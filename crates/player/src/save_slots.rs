//! Three manual save slots alongside the unchanged F5/F9 quick-save file.
//! All scans and JSON reads/writes execute in existing background workers.
use super::*;
use std::{fs, path::PathBuf};

pub const SLOT_COUNT: usize = 3;

#[derive(Clone, Debug)]
pub struct SlotInfo {
    pub exists: bool,
    pub scene: String,
    pub description: String,
}

pub fn slot_file(game_id: &str, slot: usize) -> Result<PathBuf, String> {
    if !(1..=SLOT_COUNT).contains(&slot) {
        return Err(format!("save slot {slot} out of range"));
    }
    let quick = save_file(game_id)?;
    let parent = quick.parent().ok_or("quick-save path has no parent")?;
    Ok(parent.join(format!("{game_id}.slot{slot}.json")))
}

pub fn scan(game_id: &str) -> Result<Vec<SlotInfo>, String> {
    let mut output=Vec::with_capacity(SLOT_COUNT);
    for slot in 1..=SLOT_COUNT {
        let path=slot_file(game_id,slot)?;
        let meta=match fs::metadata(&path) {
            Ok(meta)=>meta,
            Err(error) if error.kind()==std::io::ErrorKind::NotFound => {
                output.push(SlotInfo {
                    exists:false, scene:String::new(), description:"Empty".into(),
                });
                continue;
            },
            Err(error)=>{
                output.push(SlotInfo {
                    exists:true, scene:String::new(),
                    description:format!("Unavailable: {error}"),
                });
                continue;
            }
        };
        let size=meta.len();
        match SaveData::read(&path) {
            Ok(save) if save.game_id==game_id && save.format_version==1 => {
                let age=meta.modified().ok().and_then(|t|t.elapsed().ok())
                    .map(|d|{
                        if d.as_secs()<60 {"just now".to_owned()}
                        else if d.as_secs()<3600 {format!("{}m ago",d.as_secs()/60)}
                        else if d.as_secs()<86400 {format!("{}h ago",d.as_secs()/3600)}
                        else {format!("{}d ago",d.as_secs()/86400)}
                    }).unwrap_or_else(||"date unknown".into());
                output.push(SlotInfo {
                    exists:true,scene:save.machine.scene,
                    description:format!("{age} · {} KiB", (size+1023)/1024),
                });
            }
            Ok(_)=>output.push(SlotInfo {
                exists:true, scene:String::new(),
                description:"Incompatible save".into(),
            }),
            Err(error)=>output.push(SlotInfo {
                exists:true,scene:String::new(),
                description:format!("Unreadable: {}",error.chars().take(80).collect::<String>()),
            }),
        }
    }
    Ok(output)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Mode { #[default] Save, Load }

#[derive(Clone, Copy, Debug)]
pub enum Action {
    Slot(usize), Mode(Mode), Execute, Confirm, Cancel, Close,
}
#[derive(Resource, Default)]
pub struct Click(pub Option<Action>);

#[derive(Clone, Copy, Debug)]
pub enum Request { Scan, Save(usize), Load(usize) }
#[derive(Resource, Default)]
pub struct Pending(pub Option<Request>);

#[derive(Resource, Default)]
pub struct SaveUi {
    pub overlay: Option<Entity>,
    pub just_closed: bool,
    pub mode: Mode,
    pub selected: usize, // index 0..SLOT_COUNT-1
    pub confirm: Option<usize>,
    pub slots: Vec<SlotInfo>,
    pub status: String,
}
impl SaveUi {
    pub fn visible(&self)->bool{self.overlay.is_some()}
}

#[derive(Component)]
pub struct SlotRow(pub usize);
#[derive(Component)]
pub struct ModeRow;
#[derive(Component)]
pub struct StatusRow;

fn button(parent:&mut ChildSpawnerCommands, label:&str, action:Action,
    font:&FontSource, accent:[f32;4]) {
    parent.spawn((
        Name::new(format!("Save manager button: {label}")),
        Button, Hovered::default(),
        observe(move |_activate: On<Activate>,mut clicks:ResMut<Click>|{
            clicks.0=Some(action);
        }),
        Text::new(label),
        TextFont {font:font.clone(),font_size:FontSize::Px(18.0),..default()},
        TextColor(Color::WHITE),
        Node {padding:px(10.0).all(),..default()},
        BackgroundColor(Color::srgba(accent[0]*0.22,
            accent[1]*0.22,accent[2]*0.22,0.9)),
    ));
}

pub fn open(ui:&mut SaveUi,pending:&mut Pending,commands:&mut Commands,
    assets:&AssetServer,theme:&vesper_game::Theme) {
    if ui.visible(){return;}
    ui.selected=0;
    ui.mode=Mode::Save;
    ui.confirm=None;
    ui.slots.clear();
    ui.status="Scanning save slots…".into();
    let font=text_font(theme,assets);
    let accent=theme.panel.accent;
    let entity=commands.spawn((
        Name::new("VX08 save manager overlay"),
        Node {position_type:PositionType::Absolute,
            width:percent(100.0),height:percent(100.0),
            align_items:AlignItems::Center,justify_content:JustifyContent::Center,
            ..default()},
        BackgroundColor(Color::srgba(0.01,0.02,0.045,0.92)),
    )).with_children(|root|{
        root.spawn((
            Name::new("Save slot manager panel"),
            Node {width:percent(88.0),max_width:px(830.0),
                padding:px(20.0).all(),row_gap:px(9.0),
                flex_direction:FlexDirection::Column,..default()},
            BackgroundColor(Color::srgba(0.05,0.07,0.13,0.98)),
        )).with_children(|panel|{
            panel.spawn((
                Text::new("Save and load"),
                TextFont{font:font.clone(),font_size:FontSize::Px(26.0),..default()},
                TextColor(Color::srgba(accent[0],accent[1],accent[2],1.0)),
            ));
            panel.spawn((
                ModeRow,Text::new("Save mode · ←/→ to switch"),
                TextFont{font:font.clone(),font_size:FontSize::Px(18.0),..default()},
                TextColor(Color::WHITE),
            ));
            for n in 0..SLOT_COUNT {
                panel.spawn((
                    Name::new(format!("Save slot {}",n+1)),
                    SlotRow(n),
                    Button,Hovered::default(),
                    observe(move |_activate: On<Activate>,mut clicks:ResMut<Click>|{
                        clicks.0=Some(Action::Slot(n));
                    }),
                    Text::new(format!("{}. Scanning…",n+1)),
                    TextFont{font:font.clone(),font_size:FontSize::Px(19.0),..default()},
                    TextColor(Color::WHITE),
                    Node{width:percent(100.0),padding:px(9.0).all(),..default()},
                    BackgroundColor(Color::NONE),
                ));
            }
            panel.spawn((
                StatusRow,Text::new("Scanning save slots…"),
                TextFont{font:font.clone(),font_size:FontSize::Px(16.0),..default()},
                TextColor(Color::WHITE),
            ));
            panel.spawn((Node{flex_direction:FlexDirection::Row,
                column_gap:px(8.0),flex_wrap:FlexWrap::Wrap,..default()},
            )).with_children(|controls|{
                button(controls,"Save",Action::Mode(Mode::Save),&font,accent);
                button(controls,"Load",Action::Mode(Mode::Load),&font,accent);
                button(controls,"Use selected slot",Action::Execute,&font,accent);
                button(controls,"Confirm overwrite",Action::Confirm,&font,accent);
                button(controls,"Cancel",Action::Cancel,&font,accent);
                button(controls,"Close (F6 / Esc)",Action::Close,&font,accent);
            });
            panel.spawn((
                Text::new("↑↓ choose  ←→ mode  Enter use slot  Esc close  ·  F5/F9 quick save/load unchanged"),
                TextFont{font:font.clone(),font_size:FontSize::Px(14.0),..default()},
                TextColor(Color::WHITE),
                TextLayout::new(Justify::Left,LineBreak::WordOrCharacter),
            ));
        });
    }).id();
    ui.overlay=Some(entity);
    pending.0=Some(Request::Scan);
    info!("Vesper save slots opened");
}

pub fn close(ui:&mut SaveUi,commands:&mut Commands){
    if let Some(entity)=ui.overlay.take() {
        commands.entity(entity).despawn();
        ui.confirm=None;
        ui.just_closed = true;
        info!("Vesper save slots closed");
    }
}

fn execute(ui:&mut SaveUi,pending:&mut Pending){
    if ui.slots.len()!=SLOT_COUNT {
        ui.status="Save slots are still loading".into();
        return;
    }
    let slot=ui.selected+1;
    let info=&ui.slots[ui.selected];
    match ui.mode {
        Mode::Load if !info.exists => {
            ui.status=format!("Slot {slot} is empty");
        }
        Mode::Load => {
            pending.0=Some(Request::Load(slot));
            ui.status=format!("Loading slot {slot}…");
            info!("Vesper manual slot load requested: {slot}");
        }
        Mode::Save if info.exists => {
            ui.confirm=Some(slot);
            ui.status=format!("Overwrite slot {slot}? Press Enter / Confirm, or Escape.");
            info!("Vesper overwrite confirmation requested: {slot}");
        }
        Mode::Save => {
            pending.0=Some(Request::Save(slot));
            ui.status=format!("Saving slot {slot}…");
        }
    }
}

pub fn input(key:&ButtonInput<KeyCode>,ui:&mut SaveUi,
    click:&mut Click,pending:&mut Pending,commands:&mut Commands) {
    let action=click.0.take();
    if key.just_pressed(KeyCode::F6)
        || matches!(action,Some(Action::Close)) {
        close(ui,commands);return;
    }
    if ui.confirm.is_some() {
        if key.just_pressed(KeyCode::Escape)
            || matches!(action,Some(Action::Cancel)) {
            ui.confirm=None;
            ui.status="Overwrite cancelled".into();
            info!("Vesper manual overwrite cancelled");
            return;
        }
        if key.just_pressed(KeyCode::Enter)
            || matches!(action,Some(Action::Confirm)) {
            let slot=ui.confirm.take().unwrap();
            pending.0=Some(Request::Save(slot));
            ui.status=format!("Saving slot {slot}…");
            info!("Vesper overwrite confirmed: {slot}");
        }
        return;
    }
    if key.just_pressed(KeyCode::Escape){close(ui,commands);return;}
    if let Some(Action::Slot(n))=action {
        ui.selected=n.min(SLOT_COUNT-1);
    } else if let Some(Action::Mode(mode))=action {
        ui.mode=mode;
    } else if key.just_pressed(KeyCode::ArrowDown) {
        ui.selected=(ui.selected+1)%SLOT_COUNT;
    } else if key.just_pressed(KeyCode::ArrowUp) {
        ui.selected=(ui.selected+SLOT_COUNT-1)%SLOT_COUNT;
    } else if key.just_pressed(KeyCode::ArrowLeft)
        || key.just_pressed(KeyCode::ArrowRight) {
        ui.mode=if ui.mode==Mode::Save {Mode::Load} else {Mode::Save};
    }
    if key.just_pressed(KeyCode::Enter)
        || matches!(action,Some(Action::Execute)) {
        execute(ui,pending);
    }
}

pub fn sync(
    ui:Res<SaveUi>,
    session:Res<Session>,
    mut slots:Query<(&SlotRow,&mut Text,&mut BackgroundColor)>,
    mut modes:Query<&mut Text,(With<ModeRow>,Without<SlotRow>,Without<StatusRow>)>,
    mut statuses:Query<&mut Text,(With<StatusRow>,Without<SlotRow>,Without<ModeRow>)>,
) {
    if !ui.is_changed(){return;}
    if !ui.visible(){return;}
    for (row,mut text,mut bg) in &mut slots {
        let detail=ui.slots.get(row.0).map_or("Scanning…".to_owned(),|info|{
            if info.exists {
                if info.scene.is_empty(){info.description.clone()}
                else {format!("{} · {}",info.scene,info.description)}
            } else {"Empty".into()}
        });
        let label=format!("{}. {detail}",row.0+1);
        if text.0!=label{text.0=label;}
        let color=if row.0==ui.selected {
            let c=session.game.theme.panel.accent;
            Color::srgba(c[0]*0.25,c[1]*0.25,c[2]*0.25,0.95)
        }else{Color::NONE};
        if bg.0!=color{bg.0=color;}
    }
    let mode_label=format!("Mode: {}  ·  ← / → to switch",
        if ui.mode==Mode::Save{"SAVE"}else{"LOAD"});
    for mut text in &mut modes {
        if text.0!=mode_label{text.0.clone_from(&mode_label);}
    }
    for mut text in &mut statuses {
        if text.0!=ui.status{text.0.clone_from(&ui.status);}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn slot_paths_are_bounded_and_do_not_change_quick_save_name() {
        assert!(slot_file("demo",0).is_err());
        assert!(slot_file("demo",4).is_err());
        let p=slot_file("demo",2).unwrap();
        assert!(p.to_string_lossy().ends_with("demo.slot2.json"));
        let quick=save_file("demo").unwrap();
        assert!(quick.to_string_lossy().ends_with("demo.json"));
    }
}
