//! Deterministic narrative parser and interpreter, independent of graphics and frame rate.
//! The .vn format is preliminary and will be stabilized before Vesper 1.0.

use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Diagnostic {
    pub line: usize,
    pub message: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Value {
    Number(i64),
    Boolean(bool),
    Text(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Choice {
    pub label: String,
    pub destination: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Visual {
    Background(String),
    Show { character: String, expression: String, position: String },
    Hide(String),
    Music(String),
    Sound(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Output {
    Dialogue { speaker: Option<String>, text: String },
    Choices(Vec<Choice>),
    Visual(Visual),
    End,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum Command {
    Dialogue(Option<String>, String),
    Choices(Vec<Choice>),
    Visual(Visual),
    Set(String, Assign, Value),
    If(Condition, usize),
    Skip(usize),
    Jump(String),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Assign { Replace, Add, Subtract }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Compare { Eq, Ne, Lt, Le, Gt, Ge }

#[derive(Clone, Debug, PartialEq, Eq)]
struct Condition { variable: String, op: Compare, expected: Value }

#[derive(Clone, Debug, PartialEq, Eq)]
struct Instruction { line: usize, command: Command }

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct Scene { instructions: Vec<Instruction> }

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Story { scenes: BTreeMap<String, Scene> }

fn identifier(input: &str) -> bool {
    let mut chars = input.chars();
    matches!(chars.next(), Some(c) if c.is_ascii_alphabetic() || c == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.'))
}

fn quoted(input: &str) -> Result<String, String> {
    let inner = input.strip_prefix('"').and_then(|s| s.strip_suffix('"'))
        .ok_or("expected a double-quoted string")?;
    let mut chars = inner.chars();
    let mut out = String::new();
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some('"') => out.push('"'),
                Some('\\') => out.push('\\'),
                Some('n') => out.push('\n'),
                _ => return Err("invalid string escape".into()),
            }
        } else if c == '"' {
            return Err("unescaped quotation mark".into());
        } else {
            out.push(c);
        }
    }
    Ok(out)
}

fn value(input: &str) -> Result<Value, String> {
    let input = input.trim();
    match input {
        "true" => Ok(Value::Boolean(true)),
        "false" => Ok(Value::Boolean(false)),
        _ if input.starts_with('"') => quoted(input).map(Value::Text),
        _ => input.parse::<i64>().map(Value::Number)
            .map_err(|_| "expected an integer, boolean, or quoted string".into()),
    }
}

fn command(line: &str) -> Result<Command, String> {
    if let Some(rest) = line.strip_prefix("* ") {
        let (label, target) = rest.rsplit_once(" -> ").ok_or("choice needs ' -> scene'")?;
        if !identifier(target) { return Err("invalid choice destination".into()); }
        return Ok(Command::Choices(vec![Choice {
            label: quoted(label)?, destination: target.into(),
        }]));
    }
    if let Some(rest) = line.strip_prefix("jump ") {
        if !identifier(rest) { return Err("invalid jump destination".into()); }
        return Ok(Command::Jump(rest.into()));
    }
    if let Some(rest) = line.strip_prefix("if ") {
        let mut parts = rest.splitn(3, ' ');
        let (Some(variable), Some(op), Some(expected)) =
            (parts.next(), parts.next(), parts.next()) else {
                return Err("condition syntax: if variable operator literal".into());
            };
        if !identifier(variable) { return Err("invalid condition variable".into()); }
        let op = match op {
            "==" => Compare::Eq, "!=" => Compare::Ne,
            "<" => Compare::Lt, "<=" => Compare::Le,
            ">" => Compare::Gt, ">=" => Compare::Ge,
            _ => return Err("unknown comparison operator".into()),
        };
        return Ok(Command::If(Condition {
            variable: variable.into(), op, expected: value(expected)?,
        }, usize::MAX));
    }
    if let Some(rest) = line.strip_prefix("set ") {
        for (separator, op) in [
            (" += ", Assign::Add), (" -= ", Assign::Subtract), (" = ", Assign::Replace),
        ] {
            if let Some((name, expression)) = rest.split_once(separator) {
                if !identifier(name) { return Err("invalid variable identifier".into()); }
                return Ok(Command::Set(name.into(), op, value(expression)?));
            }
        }
        return Err("assignment syntax: set variable = literal".into());
    }
    for (prefix, kind) in [
        ("background ", 0), ("hide ", 1), ("music ", 2), ("sound ", 3),
    ] {
        if let Some(id) = line.strip_prefix(prefix) {
            if !identifier(id) { return Err("invalid asset or character identifier".into()); }
            let item = match kind {
                0 => Visual::Background(id.into()),
                1 => Visual::Hide(id.into()),
                2 => Visual::Music(id.into()),
                _ => Visual::Sound(id.into()),
            };
            return Ok(Command::Visual(item));
        }
    }
    if let Some(rest) = line.strip_prefix("show ") {
        let words: Vec<_> = rest.split_whitespace().collect();
        if words.len() != 4 || words[2] != "at"
            || !identifier(words[0]) || !identifier(words[1]) || !identifier(words[3]) {
            return Err("show syntax: show character expression at position".into());
        }
        return Ok(Command::Visual(Visual::Show {
            character: words[0].into(), expression: words[1].into(),
            position: words[3].into(),
        }));
    }
    if line.starts_with('"') {
        return Ok(Command::Dialogue(None, quoted(line)?));
    }
    if let Some((speaker, text)) = line.split_once(": ") {
        if identifier(speaker) {
            return Ok(Command::Dialogue(Some(speaker.into()), quoted(text)?));
        }
    }
    Err("unrecognized command or dialogue".into())
}

impl Story {
    pub fn scene_count(&self) -> usize { self.scenes.len() }

    pub fn parse(source: &str) -> Result<Self, Vec<Diagnostic>> {
        let mut scenes = BTreeMap::<String, Scene>::new();
        let mut current: Option<String> = None;
        let mut blocks: Vec<(usize, Option<usize>)> = Vec::new();
        let mut errors = Vec::new();

        for (offset, raw) in source.lines().enumerate() {
            let line_no = offset + 1;
            let line = raw.trim();
            if line.is_empty() || line.starts_with('#') { continue; }
            if let Some(name) = line.strip_prefix("@scene ") {
                if !blocks.is_empty() {
                    errors.push(Diagnostic {
                        line: line_no, message: "unclosed 'if' before next scene".into(),
                    });
                    blocks.clear();
                }
                if !identifier(name) {
                    errors.push(Diagnostic {
                        line: line_no, message: "invalid scene identifier".into(),
                    });
                    current = None;
                } else if scenes.contains_key(name) {
                    errors.push(Diagnostic {
                        line: line_no, message: format!("duplicate scene '{name}'"),
                    });
                    current = None;
                } else {
                    scenes.insert(name.into(), Scene::default());
                    current = Some(name.into());
                }
                continue;
            }

            let Some(name) = &current else {
                errors.push(Diagnostic {
                    line: line_no, message: "expected '@scene name'".into(),
                });
                continue;
            };
            let instructions = &mut scenes.get_mut(name).expect("scene exists").instructions;
            let index = instructions.len();
            if line == "else" {
                if let Some((start, alternate)) = blocks.last_mut() {
                    if alternate.is_some() {
                        errors.push(Diagnostic {
                            line: line_no, message: "duplicate 'else'".into(),
                        });
                    } else {
                        if let Command::If(_, otherwise) = &mut instructions[*start].command {
                            *otherwise = index + 1;
                        }
                        *alternate = Some(index);
                        instructions.push(Instruction {
                            line: line_no, command: Command::Skip(usize::MAX),
                        });
                    }
                } else {
                    errors.push(Diagnostic {
                        line: line_no, message: "'else' without 'if'".into(),
                    });
                }
                continue;
            }
            if line == "end" {
                if let Some((start, alternate)) = blocks.pop() {
                    match alternate {
                        Some(skip) => {
                            if let Command::Skip(target) = &mut instructions[skip].command {
                                *target = index;
                            }
                        }
                        None => {
                            if let Command::If(_, target) = &mut instructions[start].command {
                                *target = index;
                            }
                        }
                    }
                } else {
                    errors.push(Diagnostic {
                        line: line_no, message: "'end' without 'if'".into(),
                    });
                }
                continue;
            }

            match command(line) {
                Ok(Command::If(cond, _)) => {
                    blocks.push((index, None));
                    instructions.push(Instruction {
                        line: line_no, command: Command::If(cond, usize::MAX),
                    });
                }
                Ok(Command::Choices(mut options)) => {
                    if let Some(Instruction {
                        command: Command::Choices(previous), ..
                    }) = instructions.last_mut() {
                        previous.append(&mut options);
                    } else {
                        instructions.push(Instruction {
                            line: line_no, command: Command::Choices(options),
                        });
                    }
                }
                Ok(item) => instructions.push(Instruction { line: line_no, command: item }),
                Err(message) => errors.push(Diagnostic { line: line_no, message }),
            }
        }
        if !blocks.is_empty() {
            errors.push(Diagnostic {
                line: source.lines().count().max(1),
                message: "unclosed 'if' at end of file".into(),
            });
        }
        if scenes.is_empty() {
            errors.push(Diagnostic {
                line: 1, message: "story has no scenes".into(),
            });
        }
        for scene in scenes.values() {
            for entry in &scene.instructions {
                let destinations: Vec<&str> = match &entry.command {
                    Command::Jump(dest) => vec![dest],
                    Command::Choices(options) =>
                        options.iter().map(|option| option.destination.as_str()).collect(),
                    _ => Vec::new(),
                };
                for dest in destinations {
                    if !scenes.contains_key(dest) {
                        errors.push(Diagnostic {
                            line: entry.line,
                            message: format!("unknown destination scene '{dest}'"),
                        });
                    }
                }
            }
        }
        if errors.is_empty() { Ok(Self { scenes }) } else { Err(errors) }
    }
}

/// Headless story machine. Blocking output is emitted once per player advancement.
pub struct Machine<'a> {
    story: &'a Story,
    scene: String,
    cursor: usize,
    variables: BTreeMap<String, Value>,
    pending: Option<Vec<Choice>>,
}

impl<'a> Machine<'a> {
    pub fn new(story: &'a Story, scene: &str) -> Result<Self, String> {
        if !story.scenes.contains_key(scene) {
            return Err(format!("unknown starting scene '{scene}'"));
        }
        Ok(Self {
            story, scene: scene.into(), cursor: 0,
            variables: BTreeMap::new(), pending: None,
        })
    }

    pub fn scene(&self) -> &str { &self.scene }
    pub fn variables(&self) -> &BTreeMap<String, Value> { &self.variables }

    pub fn choose(&mut self, index: usize) -> Result<(), String> {
        let choices = self.pending.as_ref().ok_or("no choice is pending")?;
        let destination = choices.get(index).ok_or("choice index out of range")?.destination.clone();
        self.scene = destination;
        self.cursor = 0;
        self.pending = None;
        Ok(())
    }

    pub fn advance(&mut self) -> Result<Output, String> {
        if let Some(choices) = &self.pending {
            return Ok(Output::Choices(choices.clone()));
        }
        // An accidental authored loop cannot hang the Bevy update system forever.
        for _ in 0..10_000 {
            let scene = &self.story.scenes[&self.scene];
            let Some(entry) = scene.instructions.get(self.cursor) else {
                return Ok(Output::End);
            };
            let instruction = entry.command.clone();
            let line = entry.line;
            self.cursor += 1;
            match instruction {
                Command::Dialogue(speaker, text) =>
                    return Ok(Output::Dialogue { speaker, text }),
                Command::Choices(options) => {
                    self.pending = Some(options.clone());
                    return Ok(Output::Choices(options));
                }
                Command::Visual(visual) => return Ok(Output::Visual(visual)),
                Command::Jump(dest) => {
                    self.scene = dest;
                    self.cursor = 0;
                }
                Command::Skip(target) => self.cursor = target,
                Command::If(cond, otherwise) => {
                    if !self.condition(&cond)? { self.cursor = otherwise; }
                }
                Command::Set(name, op, value) => {
                    let value = match op {
                        Assign::Replace => value,
                        Assign::Add | Assign::Subtract => {
                            let Some(Value::Number(old)) = self.variables.get(&name) else {
                                return Err(format!("line {line}: variable '{name}' must be an initialized integer"));
                            };
                            let Value::Number(amount) = value else {
                                return Err(format!("line {line}: arithmetic needs an integer"));
                            };
                            let result = match op {
                                Assign::Add => old.checked_add(amount),
                                Assign::Subtract => old.checked_sub(amount),
                                Assign::Replace => unreachable!(),
                            };
                            Value::Number(result.ok_or(format!("line {line}: integer overflow"))?)
                        }
                    };
                    self.variables.insert(name, value);
                }
            }
        }
        Err("story exceeded 10000 steps without yielding (possible infinite loop)".into())
    }

    fn condition(&self, cond: &Condition) -> Result<bool, String> {
        let actual = self.variables.get(&cond.variable)
            .ok_or(format!("uninitialized variable '{}'", cond.variable))?;
        let expected = &cond.expected;
        let value = match cond.op {
            Compare::Eq => actual == expected,
            Compare::Ne => actual != expected,
            other => {
                let (Value::Number(a), Value::Number(b)) = (actual, expected) else {
                    return Err(format!("ordered comparison '{}' requires integers", cond.variable));
                };
                match other {
                    Compare::Lt => a < b,
                    Compare::Le => a <= b,
                    Compare::Gt => a > b,
                    Compare::Ge => a >= b,
                    _ => unreachable!(),
                }
            }
        };
        Ok(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_and_executes_choices_and_state() {
        let source = r#"@scene start
set trust = 2
elena: "Hello."
* "Stay" -> stay
* "Go" -> go
@scene stay
set trust += 1
if trust >= 3
"Trusted"
else
"Untrusted"
end
@scene go
"Gone"
"#;
        let story = Story::parse(source).unwrap();
        let mut machine = Machine::new(&story, "start").unwrap();
        assert_eq!(machine.advance().unwrap(), Output::Dialogue {
            speaker: Some("elena".into()), text: "Hello.".into(),
        });
        assert!(matches!(machine.advance().unwrap(), Output::Choices(ref v) if v.len() == 2));
        assert!(machine.choose(2).is_err());
        machine.choose(0).unwrap();
        assert_eq!(machine.advance().unwrap(), Output::Dialogue {
            speaker: None, text: "Trusted".into(),
        });
        assert_eq!(machine.variables()["trust"], Value::Number(3));
        assert_eq!(machine.advance().unwrap(), Output::End);
    }

    #[test]
    fn nested_conditions_work() {
        let story = Story::parse(r#"@scene start
set x = 0
if x > 0
"wrong"
else
if x == 0
"correct"
else
"wrong too"
end
end
"done"
"#).unwrap();
        let mut machine = Machine::new(&story, "start").unwrap();
        assert_eq!(machine.advance().unwrap(), Output::Dialogue {
            speaker: None, text: "correct".into(),
        });
        assert_eq!(machine.advance().unwrap(), Output::Dialogue {
            speaker: None, text: "done".into(),
        });
    }

    #[test]
    fn validates_destinations_with_source_lines() {
        let errors = Story::parse("@scene a\njump missing\n@scene a").unwrap_err();
        assert!(errors.iter().any(|err| err.line == 2 && err.message.contains("missing")));
        assert!(errors.iter().any(|err| err.line == 3 && err.message.contains("duplicate")));
    }

    #[test]
    fn rejects_unbalanced_blocks_and_invalid_values() {
        assert!(Story::parse("@scene a\nif x == 1\n\"unfinished\"").is_err());
        assert!(Story::parse("@scene a\nelse").is_err());
        assert!(Story::parse("@scene a\nset x = garbage").is_err());
    }

    #[test]
    fn infinite_jumps_do_not_hang() {
        let story = Story::parse("@scene a\njump a").unwrap();
        let err = Machine::new(&story, "a").unwrap().advance().unwrap_err();
        assert!(err.contains("infinite loop"));
    }

    #[test]
    fn visuals_and_unicode_remain_external_data() {
        let story = Story::parse("@scene a\nbackground room\nshow elena happy at center\n\"日本語 مرحبا\"").unwrap();
        let mut machine = Machine::new(&story, "a").unwrap();
        assert_eq!(machine.advance().unwrap(), Output::Visual(Visual::Background("room".into())));
        assert!(matches!(machine.advance().unwrap(), Output::Visual(Visual::Show { .. })));
        assert_eq!(machine.advance().unwrap(), Output::Dialogue {
            speaker: None, text: "日本語 مرحبا".into(),
        });
    }
}
