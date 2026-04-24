use std::collections::BTreeSet;
use std::io::{Stdout, Write};
use std::path::PathBuf;

use clap::Parser;
use crossterm::cursor::MoveTo;
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use crossterm::execute;
use crossterm::style::{Attribute, Print, ResetColor, SetAttribute, SetForegroundColor};
use crossterm::terminal::{self, Clear, ClearType};

use crate::cli::{AddArgs, Cli, Commands, RemoveArgs, SyncArgs};
use crate::colors::term;
use crate::commands::add::{
    config_path_for_edit, discover_available_skills, load_or_default_config, normalize_source,
    skill_names_from_field,
};
use crate::error::Result;
use crate::model::{resolve_scope, Scope, SkillsField};
use crate::tui::draw_banner_or_fallback;

pub(super) fn prompt_sync_args(
    stdout: &mut Stdout,
    program_name: &str,
    default_config: &str,
) -> Result<Option<SyncArgs>> {
    let mut input = String::new();
    let mut error = None::<String>;

    loop {
        draw_sync_prompt(
            stdout,
            program_name,
            default_config,
            &input,
            error.as_deref(),
        )?;
        match event::read()? {
            Event::Key(key) if key.kind != KeyEventKind::Release => match key.code {
                KeyCode::Enter => match parse_sync_args(program_name, &input) {
                    Ok(sync) => return Ok(Some(sync)),
                    Err(message) => error = Some(message),
                },
                KeyCode::Esc => return Ok(None),
                KeyCode::Backspace => {
                    input.pop();
                    error = None;
                }
                KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    input.clear();
                    error = None;
                }
                KeyCode::Char(ch) => {
                    input.push(ch);
                    error = None;
                }
                _ => {}
            },
            Event::Paste(text) => {
                input.push_str(&text);
                error = None;
            }
            Event::Resize(_, _) => {}
            _ => {}
        }
    }
}

pub(super) fn prompt_add_args(stdout: &mut Stdout, program_name: &str) -> Result<Option<AddArgs>> {
    let mut repo_input = String::new();
    let mut global = false;
    let mut error = None::<String>;

    loop {
        draw_add_repo_prompt(stdout, program_name, &repo_input, global, error.as_deref())?;
        match event::read()? {
            Event::Key(key) if key.kind != KeyEventKind::Release => match key.code {
                KeyCode::Enter => {
                    let repo = match normalize_source(&repo_input) {
                        Ok(repo) => repo,
                        Err(message) => {
                            error = Some(message.to_string());
                            continue;
                        }
                    };

                    draw_add_loading_prompt(stdout, program_name, &repo, global)?;

                    let available = match discover_available_skills(&repo) {
                        Ok(available) => available,
                        Err(message) => {
                            error = Some(message.to_string());
                            continue;
                        }
                    };

                    if available.is_empty() {
                        error = Some(format!("no skills found in source: {repo}"));
                        continue;
                    }

                    if let Some(skills) = prompt_add_skill_selection(
                        stdout,
                        program_name,
                        &repo,
                        &available,
                        &mut global,
                    )? {
                        return Ok(Some(AddArgs {
                            repo,
                            skills,
                            global,
                        }));
                    }

                    error = None;
                }
                KeyCode::Esc => return Ok(None),
                KeyCode::Backspace => {
                    repo_input.pop();
                    error = None;
                }
                KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    repo_input.clear();
                    error = None;
                }
                KeyCode::Char('g') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    global = !global;
                    error = None;
                }
                KeyCode::Char(ch) => {
                    repo_input.push(ch);
                    error = None;
                }
                _ => {}
            },
            Event::Paste(text) => {
                repo_input.push_str(&text);
                error = None;
            }
            Event::Resize(_, _) => {}
            _ => {}
        }
    }
}

pub(super) fn prompt_remove_args(
    stdout: &mut Stdout,
    program_name: &str,
) -> Result<Option<RemoveArgs>> {
    let mut selected = 0usize;
    let mut global = false;
    let mut error = None::<String>;

    loop {
        let loaded = load_installed_sources(global)?;
        if selected >= loaded.entries.len() {
            selected = loaded.entries.len().saturating_sub(1);
        }

        draw_remove_source_prompt(stdout, program_name, &loaded, selected, error.as_deref())?;

        match event::read()? {
            Event::Key(key) if key.kind != KeyEventKind::Release => match key.code {
                KeyCode::Up | KeyCode::Char('k') => {
                    selected = selected.saturating_sub(1);
                    error = None;
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    selected = (selected + 1).min(loaded.entries.len().saturating_sub(1));
                    error = None;
                }
                KeyCode::Enter => {
                    let Some(entry) = loaded.entries.get(selected).cloned() else {
                        continue;
                    };

                    match entry.skills {
                        InstalledSkills::All => {
                            return Ok(Some(RemoveArgs {
                                repo: entry.source,
                                skills: vec![],
                                global,
                                unattended: false,
                            }));
                        }
                        InstalledSkills::Explicit(skills) => {
                            if let Some(selected_skills) = prompt_remove_skill_selection(
                                stdout,
                                program_name,
                                &entry.source,
                                &skills,
                                &mut global,
                            )? {
                                return Ok(Some(RemoveArgs {
                                    repo: entry.source,
                                    skills: selected_skills,
                                    global,
                                    unattended: false,
                                }));
                            }
                            error = None;
                        }
                    }
                }
                KeyCode::Esc => return Ok(None),
                KeyCode::Char('g') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    global = !global;
                    selected = 0;
                    error = None;
                }
                _ => {}
            },
            Event::Resize(_, _) => {}
            _ => {}
        }
    }
}

fn draw_sync_prompt(
    stdout: &mut Stdout,
    program_name: &str,
    default_config: &str,
    input: &str,
    error: Option<&str>,
) -> Result<()> {
    let (width, height) = terminal::size()?;
    let width = width as usize;
    let height = height as usize;

    execute!(stdout, MoveTo(0, 0), Clear(ClearType::All))?;

    let title = format!("{program_name} | カセット");
    let mut row = draw_banner_or_fallback(stdout, &title, width, height, 0)?;

    execute!(
        stdout,
        MoveTo(0, row),
        SetForegroundColor(term::INFO),
        SetAttribute(Attribute::Bold),
        Print("Sync Args"),
        SetAttribute(Attribute::Reset),
        ResetColor
    )?;
    row = row.saturating_add(1);

    execute!(
        stdout,
        MoveTo(0, row),
        Print("Enter sync args exactly as you would after the binary name.")
    )?;
    row = row.saturating_add(1);

    execute!(
        stdout,
        MoveTo(0, row),
        SetForegroundColor(term::SECONDARY),
        Print(format!(
            "Example: {} --config https://example.com/kasetto.yaml --dry-run",
            program_name
        )),
        ResetColor
    )?;
    row = row.saturating_add(1);

    execute!(
        stdout,
        MoveTo(0, row),
        SetForegroundColor(term::SECONDARY),
        Print(format!(
            "Shorthand: {} \"/path/to/kasetto.yaml\" --verbose",
            program_name
        )),
        ResetColor
    )?;
    row = row.saturating_add(1);

    let cfg_scope = resolve_scope(None, None);
    execute!(
        stdout,
        MoveTo(0, row),
        SetForegroundColor(term::SECONDARY),
        Print(format!(
            "Default scope from config: {} — append --global or --project to override.",
            scope_label(cfg_scope)
        )),
        ResetColor
    )?;
    row = row.saturating_add(2);

    execute!(
        stdout,
        MoveTo(0, row),
        SetForegroundColor(term::ACCENT),
        Print("sync> "),
        ResetColor
    )?;

    if input.is_empty() {
        execute!(
            stdout,
            SetForegroundColor(term::SECONDARY),
            Print(format!("--config {}", default_config)),
            ResetColor
        )?;
    } else {
        execute!(stdout, Print(input))?;
    }
    let input_row = row;
    row = row.saturating_add(2);

    if let Some(message) = error {
        execute!(
            stdout,
            MoveTo(0, row),
            SetForegroundColor(term::ERROR),
            Print(message),
            ResetColor
        )?;
    }

    let footer_row = height.saturating_sub(2) as u16;
    let input_col = if input.is_empty() {
        6
    } else {
        6 + input.chars().count() as u16
    };
    execute!(
        stdout,
        MoveTo(0, footer_row),
        SetForegroundColor(term::SECONDARY),
        Print("Enter run   Esc cancel   Ctrl-U clear"),
        ResetColor,
        MoveTo(input_col, input_row)
    )?;

    stdout.flush()?;
    Ok(())
}

fn draw_add_repo_prompt(
    stdout: &mut Stdout,
    program_name: &str,
    input: &str,
    global: bool,
    error: Option<&str>,
) -> Result<()> {
    let (width, height) = terminal::size()?;
    let width = width as usize;
    let height = height as usize;

    execute!(stdout, MoveTo(0, 0), Clear(ClearType::All))?;

    let title = format!("{program_name} | カセット");
    let mut row = draw_banner_or_fallback(stdout, &title, width, height, 0)?;

    execute!(
        stdout,
        MoveTo(0, row),
        SetForegroundColor(term::INFO),
        SetAttribute(Attribute::Bold),
        Print("Add Args"),
        SetAttribute(Attribute::Reset),
        ResetColor
    )?;
    row = row.saturating_add(1);

    execute!(
        stdout,
        MoveTo(0, row),
        Print("Enter a repo or local source, then press Enter to discover skills.")
    )?;
    row = row.saturating_add(1);

    execute!(
        stdout,
        MoveTo(0, row),
        SetForegroundColor(term::SECONDARY),
        Print(format!(
            "Example: {} https://github.com/org/skills --skill code-reviewer",
            program_name
        )),
        ResetColor
    )?;
    row = row.saturating_add(1);

    execute!(
        stdout,
        MoveTo(0, row),
        SetForegroundColor(term::SECONDARY),
        Print(format!(
            "Target config: {} (Ctrl-G toggles)",
            config_target_label(global)
        )),
        ResetColor
    )?;
    row = row.saturating_add(1);

    execute!(
        stdout,
        MoveTo(0, row),
        SetForegroundColor(term::SECONDARY),
        Print(format!(
            "Global config: {} https://github.com/org/skills --global",
            program_name
        )),
        ResetColor
    )?;
    row = row.saturating_add(2);

    execute!(
        stdout,
        MoveTo(0, row),
        SetForegroundColor(term::ACCENT),
        Print("add> "),
        ResetColor
    )?;

    if input.is_empty() {
        execute!(
            stdout,
            SetForegroundColor(term::SECONDARY),
            Print("https://github.com/org/skills"),
            ResetColor
        )?;
    } else {
        execute!(stdout, Print(input))?;
    }
    let input_row = row;
    row = row.saturating_add(2);

    if let Some(message) = error {
        execute!(
            stdout,
            MoveTo(0, row),
            SetForegroundColor(term::ERROR),
            Print(message),
            ResetColor
        )?;
    }

    let footer_row = height.saturating_sub(2) as u16;
    let input_col = if input.is_empty() {
        5
    } else {
        5 + input.chars().count() as u16
    };
    execute!(
        stdout,
        MoveTo(0, footer_row),
        SetForegroundColor(term::SECONDARY),
        Print("Enter continue   Esc cancel   Ctrl-U clear   Ctrl-G toggle target"),
        ResetColor,
        MoveTo(input_col, input_row)
    )?;

    stdout.flush()?;
    Ok(())
}

fn draw_add_loading_prompt(
    stdout: &mut Stdout,
    program_name: &str,
    repo: &str,
    global: bool,
) -> Result<()> {
    let (width, height) = terminal::size()?;
    let width = width as usize;
    let height = height as usize;

    execute!(stdout, MoveTo(0, 0), Clear(ClearType::All))?;

    let title = format!("{program_name} | カセット");
    let mut row = draw_banner_or_fallback(stdout, &title, width, height, 0)?;

    execute!(
        stdout,
        MoveTo(0, row),
        SetForegroundColor(term::INFO),
        SetAttribute(Attribute::Bold),
        Print("Add Skills"),
        SetAttribute(Attribute::Reset),
        ResetColor
    )?;
    row = row.saturating_add(1);

    execute!(stdout, MoveTo(0, row), Print(format!("Source: {repo}")))?;
    row = row.saturating_add(1);

    execute!(
        stdout,
        MoveTo(0, row),
        SetForegroundColor(term::SECONDARY),
        Print(format!("Target config: {}", config_target_label(global))),
        ResetColor
    )?;
    row = row.saturating_add(2);

    execute!(
        stdout,
        MoveTo(0, row),
        SetForegroundColor(term::ACCENT),
        SetAttribute(Attribute::Bold),
        Print("Loading skills..."),
        SetAttribute(Attribute::Reset),
        ResetColor
    )?;

    stdout.flush()?;
    Ok(())
}

struct AddSkillPromptView<'a> {
    program_name: &'a str,
    repo: &'a str,
    global: bool,
    error: Option<&'a str>,
}

fn draw_add_skill_prompt(
    stdout: &mut Stdout,
    view: &AddSkillPromptView<'_>,
    available: &[String],
    selected: usize,
    chosen: &BTreeSet<String>,
) -> Result<()> {
    let (width, height) = terminal::size()?;
    let width = width as usize;
    let height = height as usize;

    execute!(stdout, MoveTo(0, 0), Clear(ClearType::All))?;

    let title = format!("{} | カセット", view.program_name);
    let mut row = draw_banner_or_fallback(stdout, &title, width, height, 0)?;

    execute!(
        stdout,
        MoveTo(0, row),
        SetForegroundColor(term::INFO),
        SetAttribute(Attribute::Bold),
        Print("Add Skills"),
        SetAttribute(Attribute::Reset),
        ResetColor
    )?;
    row = row.saturating_add(1);

    execute!(stdout, MoveTo(0, row), Print(format!("Source: {}", view.repo)))?;
    row = row.saturating_add(1);

    execute!(
        stdout,
        MoveTo(0, row),
        SetForegroundColor(term::SECONDARY),
        Print("Select skills with Space. Press Right to select all."),
        ResetColor
    )?;
    row = row.saturating_add(1);

    execute!(
        stdout,
        MoveTo(0, row),
        SetForegroundColor(term::SECONDARY),
        Print(format!(
            "Target config: {} (Ctrl-G toggles)",
            config_target_label(view.global)
        )),
        ResetColor
    )?;
    row = row.saturating_add(2);

    execute!(
        stdout,
        MoveTo(0, row),
        SetForegroundColor(term::SECONDARY),
        Print(format!("Selected: {} of {}", chosen.len(), available.len())),
        ResetColor
    )?;
    row = row.saturating_add(2);

    let max_items = height.saturating_sub(row as usize + 3).max(1);
    let (start, end) = visible_window(selected, available.len(), max_items);
    for (index, skill) in available[start..end].iter().enumerate() {
        let actual = start + index;
        execute!(stdout, MoveTo(0, row))?;
        if actual == selected {
            execute!(
                stdout,
                SetForegroundColor(term::ACCENT),
                SetAttribute(Attribute::Bold),
                Print("› "),
                SetAttribute(Attribute::Reset),
                ResetColor
            )?;
        } else {
            execute!(
                stdout,
                SetForegroundColor(term::SECONDARY),
                Print("  "),
                ResetColor
            )?;
        }

        let marker = if chosen.contains(skill) { "[x]" } else { "[ ]" };
        execute!(stdout, Print(format!("{marker} {skill}")))?;
        row = row.saturating_add(1);
    }

    if let Some(message) = view.error {
        execute!(
            stdout,
            MoveTo(0, row),
            SetForegroundColor(term::ERROR),
            Print(message),
            ResetColor
        )?;
    }

    let footer_row = height.saturating_sub(2) as u16;
    execute!(
        stdout,
        MoveTo(0, footer_row),
        SetForegroundColor(term::SECONDARY),
        Print("Enter add   Space toggle   Right select all   Left clear all   Esc back   Ctrl-G toggle target   ↑/↓ or j/k move"),
        ResetColor
    )?;

    stdout.flush()?;
    Ok(())
}

fn prompt_add_skill_selection(
    stdout: &mut Stdout,
    program_name: &str,
    repo: &str,
    available: &[String],
    global: &mut bool,
) -> Result<Option<Vec<String>>> {
    let mut selected = 0usize;
    let mut chosen = BTreeSet::new();
    let mut error = None::<String>;

    loop {
        draw_add_skill_prompt(
            stdout,
            &AddSkillPromptView {
                program_name,
                repo,
                global: *global,
                error: error.as_deref(),
            },
            available,
            selected,
            &chosen,
        )?;

        match event::read()? {
            Event::Key(key) if key.kind != KeyEventKind::Release => match key.code {
                KeyCode::Up | KeyCode::Char('k') => {
                    selected = selected.saturating_sub(1);
                    error = None;
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    selected = (selected + 1).min(available.len().saturating_sub(1));
                    error = None;
                }
                KeyCode::Char(' ') => {
                    let skill = available[selected].clone();
                    if !chosen.insert(skill.clone()) {
                        chosen.remove(&skill);
                    }
                    error = None;
                }
                KeyCode::Right => {
                    chosen = available.iter().cloned().collect();
                    error = None;
                }
                KeyCode::Left => {
                    chosen.clear();
                    error = None;
                }
                KeyCode::Enter => {
                    if chosen.is_empty() {
                        error =
                            Some("Select one or more skills, or press Right to select all.".into());
                    } else if chosen.len() == available.len() {
                        return Ok(Some(vec![]));
                    } else {
                        return Ok(Some(chosen.iter().cloned().collect()));
                    }
                }
                KeyCode::Esc => return Ok(None),
                KeyCode::Char('g') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    *global = !*global;
                    error = None;
                }
                _ => {}
            },
            Event::Resize(_, _) => {}
            _ => {}
        }
    }
}

#[derive(Clone)]
struct LoadedSources {
    path: PathBuf,
    global: bool,
    entries: Vec<InstalledSource>,
}

#[derive(Clone)]
struct InstalledSource {
    source: String,
    skills: InstalledSkills,
}

#[derive(Clone)]
enum InstalledSkills {
    All,
    Explicit(Vec<String>),
}

fn load_installed_sources(global: bool) -> Result<LoadedSources> {
    let path = config_path_for_edit(global)?;
    let cfg = load_or_default_config(&path)?;
    let mut entries = cfg
        .skills
        .into_iter()
        .map(|entry| InstalledSource {
            source: normalize_source(&entry.source)
                .unwrap_or_else(|_| entry.source.trim().to_string()),
            skills: match entry.skills {
                SkillsField::Wildcard(_) => InstalledSkills::All,
                SkillsField::List(items) => {
                    let mut skills = skill_names_from_field(&SkillsField::List(items))
                        .into_iter()
                        .collect::<Vec<_>>();
                    skills.sort();
                    InstalledSkills::Explicit(skills)
                }
            },
        })
        .collect::<Vec<_>>();
    entries.sort_by(|left, right| left.source.cmp(&right.source));

    Ok(LoadedSources {
        path,
        global,
        entries,
    })
}

fn draw_remove_source_prompt(
    stdout: &mut Stdout,
    program_name: &str,
    loaded: &LoadedSources,
    selected: usize,
    error: Option<&str>,
) -> Result<()> {
    let (width, height) = terminal::size()?;
    let width = width as usize;
    let height = height as usize;

    execute!(stdout, MoveTo(0, 0), Clear(ClearType::All))?;

    let title = format!("{program_name} | カセット");
    let mut row = draw_banner_or_fallback(stdout, &title, width, height, 0)?;

    execute!(
        stdout,
        MoveTo(0, row),
        SetForegroundColor(term::INFO),
        SetAttribute(Attribute::Bold),
        Print("Remove Source"),
        SetAttribute(Attribute::Reset),
        ResetColor
    )?;
    row = row.saturating_add(1);

    execute!(
        stdout,
        MoveTo(0, row),
        Print(format!("Config: {}", loaded.path.display()))
    )?;
    row = row.saturating_add(1);

    execute!(
        stdout,
        MoveTo(0, row),
        SetForegroundColor(term::SECONDARY),
        Print(format!(
            "Target config: {} (Ctrl-G toggles)",
            config_target_label(loaded.global)
        )),
        ResetColor
    )?;
    row = row.saturating_add(2);

    if loaded.entries.is_empty() {
        execute!(
            stdout,
            MoveTo(0, row),
            SetForegroundColor(term::SECONDARY),
            Print("No configured sources found. Toggle target with Ctrl-G or press Esc."),
            ResetColor
        )?;
        row = row.saturating_add(2);
    } else {
        execute!(
            stdout,
            MoveTo(0, row),
            Print("Select a source to remove or inspect its configured skills:")
        )?;
        row = row.saturating_add(1);

        let max_items = height.saturating_sub(row as usize + 3);
        let (start, end) = visible_window(selected, loaded.entries.len(), max_items.max(1));
        for (index, entry) in loaded.entries[start..end].iter().enumerate() {
            let actual = start + index;
            execute!(stdout, MoveTo(0, row))?;
            if actual == selected {
                execute!(
                    stdout,
                    SetForegroundColor(term::ACCENT),
                    SetAttribute(Attribute::Bold),
                    Print("› "),
                    Print(&entry.source),
                    SetAttribute(Attribute::Reset),
                    ResetColor
                )?;
            } else {
                execute!(
                    stdout,
                    SetForegroundColor(term::SECONDARY),
                    Print("  "),
                    ResetColor,
                    Print(&entry.source)
                )?;
            }

            execute!(
                stdout,
                SetForegroundColor(term::SECONDARY),
                Print(format!("  ({})", installed_summary(&entry.skills))),
                ResetColor
            )?;
            row = row.saturating_add(1);
        }
    }

    if let Some(message) = error {
        execute!(
            stdout,
            MoveTo(0, row),
            SetForegroundColor(term::ERROR),
            Print(message),
            ResetColor
        )?;
    }

    let footer_row = height.saturating_sub(2) as u16;
    execute!(
        stdout,
        MoveTo(0, footer_row),
        SetForegroundColor(term::SECONDARY),
        Print("Enter continue   Esc cancel   Ctrl-G toggle target   ↑/↓ or j/k move"),
        ResetColor
    )?;

    stdout.flush()?;
    Ok(())
}

fn prompt_remove_skill_selection(
    stdout: &mut Stdout,
    program_name: &str,
    source: &str,
    skills: &[String],
    global: &mut bool,
) -> Result<Option<Vec<String>>> {
    let mut selected = 0usize;
    let mut chosen = BTreeSet::new();
    let mut error = None::<String>;

    loop {
        draw_remove_skill_prompt(
            stdout,
            &RemoveSkillPromptView {
                program_name,
                source,
                global: *global,
                error: error.as_deref(),
            },
            skills,
            selected,
            &chosen,
        )?;

        match event::read()? {
            Event::Key(key) if key.kind != KeyEventKind::Release => match key.code {
                KeyCode::Up | KeyCode::Char('k') => {
                    selected = selected.saturating_sub(1);
                    error = None;
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    selected = (selected + 1).min(skills.len());
                    error = None;
                }
                KeyCode::Char(' ') if selected > 0 => {
                    let skill = skills[selected - 1].clone();
                    if !chosen.insert(skill.clone()) {
                        chosen.remove(&skill);
                    }
                    error = None;
                }
                KeyCode::Enter => {
                    if selected == 0 {
                        return Ok(Some(vec![]));
                    }
                    if chosen.is_empty() {
                        error = Some("Select one or more skills, or choose All skills.".into());
                    } else {
                        return Ok(Some(chosen.iter().cloned().collect()));
                    }
                }
                KeyCode::Esc => return Ok(None),
                KeyCode::Char('g') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    *global = !*global;
                    error = None;
                }
                _ => {}
            },
            Event::Resize(_, _) => {}
            _ => {}
        }
    }
}

struct RemoveSkillPromptView<'a> {
    program_name: &'a str,
    source: &'a str,
    global: bool,
    error: Option<&'a str>,
}

fn draw_remove_skill_prompt(
    stdout: &mut Stdout,
    view: &RemoveSkillPromptView<'_>,
    skills: &[String],
    selected: usize,
    chosen: &BTreeSet<String>,
) -> Result<()> {
    let (width, height) = terminal::size()?;
    let width = width as usize;
    let height = height as usize;

    execute!(stdout, MoveTo(0, 0), Clear(ClearType::All))?;

    let title = format!("{} | カセット", view.program_name);
    let mut row = draw_banner_or_fallback(stdout, &title, width, height, 0)?;

    execute!(
        stdout,
        MoveTo(0, row),
        SetForegroundColor(term::INFO),
        SetAttribute(Attribute::Bold),
        Print("Remove Skills"),
        SetAttribute(Attribute::Reset),
        ResetColor
    )?;
    row = row.saturating_add(1);

    execute!(stdout, MoveTo(0, row), Print(format!("Source: {}", view.source)))?;
    row = row.saturating_add(1);

    execute!(
        stdout,
        MoveTo(0, row),
        SetForegroundColor(term::SECONDARY),
        Print(format!(
            "Target config: {} (Ctrl-G toggles)",
            config_target_label(view.global)
        )),
        ResetColor
    )?;
    row = row.saturating_add(2);

    execute!(
        stdout,
        MoveTo(0, row),
        Print(
            "Choose All skills to remove the whole source, or select individual skills with Space:"
        )
    )?;
    row = row.saturating_add(1);

    let mut items = vec![("All skills (remove source entry)".to_string(), false)];
    items.extend(
        skills
            .iter()
            .map(|skill| (skill.clone(), chosen.contains(skill))),
    );

    let max_items = height.saturating_sub(row as usize + 3).max(1);
    let (start, end) = visible_window(selected, items.len(), max_items);
    for (index, (label, checked)) in items[start..end].iter().enumerate() {
        let actual = start + index;
        execute!(stdout, MoveTo(0, row))?;
        if actual == selected {
            execute!(
                stdout,
                SetForegroundColor(term::ACCENT),
                SetAttribute(Attribute::Bold),
                Print("› "),
                SetAttribute(Attribute::Reset),
                ResetColor
            )?;
        } else {
            execute!(
                stdout,
                SetForegroundColor(term::SECONDARY),
                Print("  "),
                ResetColor
            )?;
        }

        let marker = if actual == 0 {
            "( )"
        } else if *checked {
            "[x]"
        } else {
            "[ ]"
        };
        execute!(stdout, Print(format!("{marker} {label}")))?;
        row = row.saturating_add(1);
    }

    if let Some(message) = view.error {
        execute!(
            stdout,
            MoveTo(0, row),
            SetForegroundColor(term::ERROR),
            Print(message),
            ResetColor
        )?;
    }

    let footer_row = height.saturating_sub(2) as u16;
    execute!(
        stdout,
        MoveTo(0, footer_row),
        SetForegroundColor(term::SECONDARY),
        Print("Enter confirm   Space toggle   Esc back   Ctrl-G toggle target   ↑/↓ or j/k move"),
        ResetColor
    )?;

    stdout.flush()?;
    Ok(())
}

fn installed_summary(skills: &InstalledSkills) -> String {
    match skills {
        InstalledSkills::All => "all skills".into(),
        InstalledSkills::Explicit(skills) => {
            format!("{} skill{}", skills.len(), pluralize(skills.len()))
        }
    }
}

fn visible_window(selected: usize, total: usize, max_items: usize) -> (usize, usize) {
    if total <= max_items {
        return (0, total);
    }

    let half = max_items / 2;
    let mut start = selected.saturating_sub(half);
    if start + max_items > total {
        start = total.saturating_sub(max_items);
    }
    (start, (start + max_items).min(total))
}

fn config_target_label(global: bool) -> &'static str {
    if global {
        "global"
    } else {
        "local"
    }
}

fn pluralize(count: usize) -> &'static str {
    if count == 1 {
        ""
    } else {
        "s"
    }
}

fn parse_sync_args(program_name: &str, input: &str) -> std::result::Result<SyncArgs, String> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err("Enter sync args or a config path to continue.".into());
    }

    let mut tokens = shlex::split(trimmed)
        .ok_or_else(|| "Could not parse sync args. Check quotes and escaping.".to_string())?;

    if matches!(tokens.first().map(String::as_str), Some("sync")) {
        tokens.remove(0);
    }

    if matches!(tokens.first().map(String::as_str), Some(first) if !first.starts_with('-')) {
        tokens.insert(0, "--config".into());
    }

    let argv = std::iter::once(program_name.to_string())
        .chain(std::iter::once("sync".to_string()))
        .chain(tokens)
        .collect::<Vec<_>>();

    let cli = Cli::try_parse_from(argv).map_err(|err| err.to_string())?;
    match cli.command {
        Some(Commands::Sync { sync }) => Ok(sync),
        _ => Err("Sync args did not resolve to the sync command.".into()),
    }
}

#[cfg(test)]
fn parse_add_args(program_name: &str, input: &str) -> std::result::Result<AddArgs, String> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err("Enter a repo or add args to continue.".into());
    }

    let mut tokens = shlex::split(trimmed)
        .ok_or_else(|| "Could not parse add args. Check quotes and escaping.".to_string())?;

    if matches!(tokens.first().map(String::as_str), Some("add")) {
        tokens.remove(0);
    }

    let argv = std::iter::once(program_name.to_string())
        .chain(std::iter::once("add".to_string()))
        .chain(tokens)
        .collect::<Vec<_>>();

    let cli = Cli::try_parse_from(argv).map_err(|err| err.to_string())?;
    match cli.command {
        Some(Commands::Add { add }) => Ok(add),
        _ => Err("Add args did not resolve to the add command.".into()),
    }
}

#[cfg(test)]
fn parse_remove_args(program_name: &str, input: &str) -> std::result::Result<RemoveArgs, String> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err("Enter a repo or remove args to continue.".into());
    }

    let mut tokens = shlex::split(trimmed)
        .ok_or_else(|| "Could not parse remove args. Check quotes and escaping.".to_string())?;

    if matches!(tokens.first().map(String::as_str), Some("remove")) {
        tokens.remove(0);
    }

    let argv = std::iter::once(program_name.to_string())
        .chain(std::iter::once("remove".to_string()))
        .chain(tokens)
        .collect::<Vec<_>>();

    let cli = Cli::try_parse_from(argv).map_err(|err| err.to_string())?;
    match cli.command {
        Some(Commands::Remove { remove }) => Ok(remove),
        _ => Err("Remove args did not resolve to the remove command.".into()),
    }
}

fn scope_label(scope: Scope) -> &'static str {
    match scope {
        Scope::Global => "global",
        Scope::Project => "project",
    }
}

#[cfg(test)]
mod tests {
    use super::{parse_add_args, parse_remove_args, parse_sync_args};

    #[test]
    fn parse_sync_args_accepts_shorthand_config_path() {
        let sync = parse_sync_args("kasetto", "kasetto.yaml --dry-run").expect("sync args");
        assert_eq!(sync.config.as_deref(), Some("kasetto.yaml"));
        assert!(sync.dry_run);
    }

    #[test]
    fn parse_sync_args_accepts_explicit_sync_command() {
        let sync =
            parse_sync_args("kasetto", "sync --config remote.yaml --verbose").expect("sync args");
        assert_eq!(sync.config.as_deref(), Some("remote.yaml"));
        assert!(sync.verbose);
    }

    #[test]
    fn parse_sync_args_accepts_scope_flags() {
        let sync =
            parse_sync_args("kasetto", "--config foo.yaml --project --dry-run").expect("sync");
        assert_eq!(sync.config.as_deref(), Some("foo.yaml"));
        assert!(sync.dry_run);
        assert!(sync.scope.project);
        assert!(!sync.scope.global);
    }

    #[test]
    fn parse_add_args_accepts_repo_only() {
        let add = parse_add_args("kasetto", "https://github.com/org/skills").expect("add");
        assert_eq!(add.repo, "https://github.com/org/skills");
        assert!(add.skills.is_empty());
        assert!(!add.global);
    }

    #[test]
    fn parse_add_args_accepts_explicit_command_and_skills() {
        let add = parse_add_args(
            "kasetto",
            "add https://github.com/org/skills --skill code-reviewer --skill docs --global",
        )
        .expect("add");
        assert_eq!(add.repo, "https://github.com/org/skills");
        assert_eq!(add.skills, vec!["code-reviewer", "docs"]);
        assert!(add.global);
    }

    #[test]
    fn parse_remove_args_accepts_repo_only() {
        let remove = parse_remove_args("kasetto", "https://github.com/org/skills").expect("remove");
        assert_eq!(remove.repo, "https://github.com/org/skills");
        assert!(remove.skills.is_empty());
        assert!(!remove.global);
        assert!(!remove.unattended);
    }

    #[test]
    fn parse_remove_args_accepts_command_skills_and_unattended() {
        let remove = parse_remove_args(
            "kasetto",
            "remove https://github.com/org/skills --skill code-reviewer -u --global",
        )
        .expect("remove");
        assert_eq!(remove.repo, "https://github.com/org/skills");
        assert_eq!(remove.skills, vec!["code-reviewer"]);
        assert!(remove.global);
        assert!(remove.unattended);
    }
}
