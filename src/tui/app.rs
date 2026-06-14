use crate::agent::{Agent, AgentEvent};
use crate::config::Config;
use crate::llm;
use crate::session::SessionStore;
use crate::mcp;
use crate::tools;
use anyhow::Result;
use ratatui::{
    Terminal,
    backend::CrosstermBackend,
    widgets::{Block, Borders, Paragraph},
    layout::{Layout, Direction, Constraint, Rect},
    style::{Style, Color, Modifier},
    text::{Line, Span, Text},
};
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use syntect::{
    highlighting::ThemeSet,
    parsing::SyntaxSet,
    easy::HighlightLines,
};
use std::io;
use tokio::sync::mpsc;
use uuid::Uuid;

const STATUS_READY: &str = "Ready. Type a prompt and press Enter.";
const PROMPT_SYMBOL: &str = "❯ ";

pub struct App {
    config: Config,
    input: String,
    cursor: usize,
    messages: Vec<ChatMessage>,
    status: String,
    scroll: usize,
    streaming: bool,
    syntax_set: SyntaxSet,
    theme_set: ThemeSet,
    session_id: String,
    store: Option<SessionStore>,
    mcp_connections: Vec<McpConnection>,
}

pub struct McpConnection {
    pub client: std::sync::Arc<tokio::sync::Mutex<mcp::McpClient>>,
    pub tools: Vec<mcp::McpTool>,
}

struct ChatMessage {
    role: String,
    content: String,
}

impl App {
    pub fn new(config: &Config, mcp_connections: Vec<McpConnection>) -> Self {
        let store = SessionStore::new(&config.session.db_path).ok();
        let session_id = Uuid::new_v4().to_string();

        // Try to resume last session
        if let Some(ref s) = store {
            if let Ok(Some(last_id)) = s.last_session() {
                if let Ok(msgs) = s.load_messages(&last_id) {
                    let chat_msgs: Vec<ChatMessage> = msgs.into_iter()
                        .map(|m| ChatMessage {
                            role: m.role,
                            content: m.content,
                        })
                        .collect();
                    if !chat_msgs.is_empty() {
                        return Self {
                            config: config.clone(),
                            input: String::new(),
                            cursor: 0,
                            messages: chat_msgs,
                            status: format!("Resumed session {}", &last_id[..8]),
                            scroll: 0,
                            streaming: false,
                            syntax_set: SyntaxSet::load_defaults_newlines(),
                            theme_set: ThemeSet::load_defaults(),
                            session_id: last_id,
                            store,
                            mcp_connections,
                        };
                    }
                }
            }
        }

        // New session
        if let Some(ref s) = store {
            let _ = s.create_session(&session_id);
        }

        Self {
            config: config.clone(),
            input: String::new(),
            cursor: 0,
            messages: Vec::new(),
            status: STATUS_READY.to_string(),
            scroll: 0,
            streaming: false,
            syntax_set: SyntaxSet::load_defaults_newlines(),
            theme_set: ThemeSet::load_defaults(),
            session_id,
            store,
            mcp_connections,
        }
    }

    fn save_message(&self, role: &str, content: &str) {
        if let Some(ref store) = self.store {
            let _ = store.save_message(&self.session_id, role, content);
        }
    }

    fn new_session(&mut self) {
        let id = Uuid::new_v4().to_string();
        self.session_id = id.clone();
        self.messages.clear();
        self.status = format!("New session {}", &id[..8]);
        if let Some(ref store) = self.store {
            let _ = store.create_session(&id);
        }
    }

    pub fn run(&mut self, terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> Result<()> {
        let (event_tx, mut event_rx) = mpsc::unbounded_channel::<AgentEvent>();

        loop {
            terminal.draw(|f| {
                let area = f.area();
                let session_label = &self.session_id[..8];
                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Min(1),
                        Constraint::Length(1),
                        Constraint::Length(3),
                    ])
                    .split(area);

                self.render_chat(f, chunks[0], session_label);
                self.render_status(f, chunks[1]);
                self.render_input(f, chunks[2]);
            })?;

            if event::poll(std::time::Duration::from_millis(50))? {
                if let Event::Key(key) = event::read()? {
                    if key.kind == KeyEventKind::Press {
                        match key.code {
                            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                                if self.streaming {
                                    self.streaming = false;
                                    self.status = "Cancelled.".to_string();
                                } else {
                                    break;
                                }
                            }
                            KeyCode::Char('s') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                                self.status = format!("Session {} saved", &self.session_id[..8]);
                            }
                            KeyCode::Char('n') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                                self.new_session();
                            }
                            KeyCode::Char('l') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                                self.show_session_list();
                            }
                            KeyCode::Char(c) => {
                                self.input.insert(self.cursor, c);
                                self.cursor += 1;
                            }
                            KeyCode::Backspace => {
                                if self.cursor > 0 {
                                    self.cursor -= 1;
                                    self.input.remove(self.cursor);
                                }
                            }
                            KeyCode::Delete => {
                                if self.cursor < self.input.len() {
                                    self.input.remove(self.cursor);
                                }
                            }
                            KeyCode::Left => {
                                self.cursor = self.cursor.saturating_sub(1);
                            }
                            KeyCode::Right => {
                                if self.cursor < self.input.len() {
                                    self.cursor += 1;
                                }
                            }
                            KeyCode::Home => self.cursor = 0,
                            KeyCode::End => self.cursor = self.input.len(),
                            KeyCode::Up => {
                                self.scroll = self.scroll.saturating_add(1);
                            }
                            KeyCode::Down => {
                                self.scroll = self.scroll.saturating_sub(1);
                            }
                            KeyCode::PageUp => {
                                self.scroll = self.scroll.saturating_add(10);
                            }
                            KeyCode::PageDown => {
                                self.scroll = self.scroll.saturating_sub(10);
                            }
                            KeyCode::Enter => {
                                if !self.input.trim().is_empty() {
                                    let prompt = std::mem::take(&mut self.input);
                                    self.cursor = 0;
                                    self.messages.push(ChatMessage {
                                        role: "user".to_string(),
                                        content: prompt.clone(),
                                    });
                                    self.save_message("user", &prompt);
                                    self.streaming = true;
                                    self.status = "Thinking...".to_string();
                                    self.scroll = 0;

                                    let provider = match llm::create_provider(&self.config.llm) {
                                        Ok(p) => p,
                                        Err(e) => {
                                            self.status = format!("Error: {}", e);
                                            self.streaming = false;
                                            continue;
                                        }
                                    };
                                    let mut registry = tools::registry::builtin_tools();
                                    for conn in &self.mcp_connections {
                                        for mt in &conn.tools {
                                            registry.register(Box::new(mcp::McpToolAdapter::new(
                                                std::sync::Arc::clone(&conn.client),
                                                mt.clone(),
                                            )));
                                        }
                                    }
                                    let mut agent = Agent::new(provider, registry, &self.config);
                                    let tx = event_tx.clone();

                                    tokio::spawn(async move {
                                        match agent.run(&prompt, Some(tx.clone())).await {
                                            Ok(_) => {
                                                let _ = tx.send(AgentEvent::Done);
                                            }
                                            Err(e) => {
                                                let _ = tx.send(AgentEvent::Error(e.to_string()));
                                            }
                                        }
                                    });
                                }
                            }
                            KeyCode::Esc => {
                                break;
                            }
                            _ => {}
                        }
                    }
                }
            }

            while let Ok(event) = event_rx.try_recv() {
                match event {
                    AgentEvent::TextDelta(text) => {
                        if self.messages.last()
                            .map(|m| m.role.as_str() == "assistant")
                            .unwrap_or(false)
                        {
                            self.messages.last_mut().unwrap().content.push_str(&text);
                        } else {
                            // First chunk — save previous assistant message content
                            self.messages.push(ChatMessage {
                                role: "assistant".to_string(),
                                content: text,
                            });
                        }
                        self.scroll = 0;
                    }
                    AgentEvent::ToolCallStart(tc) => {
                        self.status = format!("⚙ {}({})", tc.name, tc.arguments);
                        self.messages.push(ChatMessage {
                            role: "tool".to_string(),
                            content: format!("⚙ {}...", tc.name),
                        });
                    }
                    AgentEvent::ToolCallEnd { name, .. } => {
                        self.status = format!("✓ {} completed", name);
                        if self.messages.last()
                            .map(|m| m.role.as_str() == "tool" && m.content.contains(&name))
                            .unwrap_or(false)
                        {
                            self.messages.last_mut().unwrap().content =
                                format!("✓ {} completed", name);
                        }
                    }
                    AgentEvent::Error(err) => {
                        self.status = format!("Error: {}", err);
                        self.messages.push(ChatMessage {
                            role: "error".to_string(),
                            content: err,
                        });
                        self.streaming = false;
                    }
                    AgentEvent::Done => {
                        // Save the assistant response to storage
                        if let Some(msg) = self.messages.last() {
                            if msg.role == "assistant" {
                                self.save_message("assistant", &msg.content);
                            }
                        }
                        self.status = STATUS_READY.to_string();
                        self.streaming = false;
                    }
                    _ => {}
                }
            }
        }

        Ok(())
    }

    fn show_session_list(&self) {
        if let Some(ref store) = self.store {
            if let Ok(sessions) = store.list_sessions() {
                for s in &sessions {
                    let current = if s.id == self.session_id { " ←" } else { "" };
                    let title = s.title.as_deref().unwrap_or("untitled");
                    println!("  {:.8} {} ({} msgs){}", s.id, title, s.message_count, current);
                }
            }
        }
    }

    fn render_chat(&self, f: &mut ratatui::Frame, area: Rect, session_label: &str) {
        let block = Block::default()
            .borders(Borders::TOP)
            .title(format!(" Chat  [{}]", session_label))
            .style(Style::default());
        let inner = block.inner(area);
        f.render_widget(block, area);

        let mut lines: Vec<Line> = Vec::new();

        for msg in &self.messages {
            let role_style = match msg.role.as_str() {
                "user" => Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
                "assistant" => Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
                "tool" => Style::default().fg(Color::Yellow),
                "error" => Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
                _ => Style::default(),
            };

            let role_label = match msg.role.as_str() {
                "user" => "You",
                "assistant" => "aibsd",
                "tool" => "Tool",
                "error" => "Error",
                _ => &msg.role,
            };

            if msg.role == "tool" || msg.role == "error" {
                lines.push(Line::from(Span::styled(
                    format!(" {} {}", role_label, msg.content),
                    role_style,
                )));
            } else {
                lines.push(Line::from(Span::styled(
                    format!(" {} ", role_label),
                    role_style,
                )));
                let content_lines = self.render_markdown(&msg.content, inner.width as usize);
                lines.extend(content_lines);
                lines.push(Line::from(""));
            }
        }

        let available = inner.height as usize;
        let total = lines.len();
        let scroll = self.scroll.min(total.saturating_sub(available));

        let visible: Vec<Line> = if total > available {
            let end = total - scroll;
            let start = end.saturating_sub(available);
            lines[start..end].to_vec()
        } else {
            lines.clone()
        };

        let paragraph = Paragraph::new(Text::from(visible))
            .style(Style::default())
            .block(Block::default());
        f.render_widget(paragraph, inner);
    }

    fn render_markdown(&self, text: &str, max_width: usize) -> Vec<Line<'static>> {
        let mut lines = Vec::new();
        if text.is_empty() { return lines; }

        let mut in_code_block = false;
        let mut code_lang = String::new();
        let mut code_lines: Vec<String> = Vec::new();

        for line in text.lines() {
            if line.starts_with("```") {
                if in_code_block {
                    let highlighted = self.highlight_code(&code_lines.join("\n"), &code_lang);
                    for hl_line in highlighted { lines.push(hl_line); }
                    code_lines.clear();
                    code_lang.clear();
                    in_code_block = false;
                } else {
                    in_code_block = true;
                    code_lang = line.trim_start_matches("```").trim().to_string();
                }
                continue;
            }

            if in_code_block {
                code_lines.push(line.to_string());
                continue;
            }

            if line.starts_with("### ") {
                lines.push(Line::from(Span::styled(line[4..].to_string(),
                    Style::default().fg(Color::LightCyan).add_modifier(Modifier::BOLD))));
            } else if line.starts_with("## ") {
                lines.push(Line::from(Span::styled(line[3..].to_string(),
                    Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))));
            } else if line.starts_with("# ") {
                lines.push(Line::from(Span::styled(line[2..].to_string(),
                    Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD | Modifier::UNDERLINED))));
            } else if line.starts_with("- ") || line.starts_with("* ") {
                lines.push(Line::from(Span::styled(format!(" • {}", &line[2..]), Style::default())));
            } else if line.starts_with("    ") || line.starts_with("\t") {
                lines.push(Line::from(Span::styled(line.to_string(), Style::default().fg(Color::DarkGray))));
            } else if line.trim().is_empty() {
                lines.push(Line::from(""));
            } else {
                if line.len() > max_width && max_width > 20 {
                    for wrapped in self.wrap_text(line, max_width - 2) {
                        lines.push(Line::from(Span::styled(wrapped, Style::default())));
                    }
                } else {
                    lines.push(Line::from(Span::styled(line.to_string(), Style::default())));
                }
            }
        }

        if !code_lines.is_empty() {
            let highlighted = self.highlight_code(&code_lines.join("\n"), &code_lang);
            for hl_line in highlighted { lines.push(hl_line); }
        }

        lines
    }

    fn highlight_code(&self, code: &str, lang: &str) -> Vec<Line<'static>> {
        let mut result = Vec::new();
        if code.is_empty() { return result; }

        let syntax = if lang.is_empty() {
            self.syntax_set.find_syntax_plain_text()
        } else {
            self.syntax_set.find_syntax_by_token(lang)
                .unwrap_or_else(|| self.syntax_set.find_syntax_plain_text())
        };

        let theme = self.theme_set.themes.get("base16-ocean.dark")
            .unwrap_or_else(|| self.theme_set.themes.values().next().unwrap());

        let mut highlighter = HighlightLines::new(syntax, theme);
        for line in code.lines() {
            let mut spans = Vec::new();
            if let Ok(ranges) = highlighter.highlight_line(line, &self.syntax_set) {
                for (style, text) in ranges {
                    let fg = style.foreground;
                    spans.push(Span::styled(text.to_string(), Style::default().fg(Color::Rgb(fg.r, fg.g, fg.b))));
                }
            }
            if !spans.is_empty() {
                result.push(Line::from(spans));
            } else {
                result.push(Line::from(Span::styled(line.to_string(), Style::default().fg(Color::DarkGray))));
            }
        }
        result
    }

    fn wrap_text(&self, text: &str, max_width: usize) -> Vec<String> {
        let mut result = Vec::new();
        let mut current = String::new();
        for word in text.split(' ') {
            if current.len() + word.len() + 1 > max_width && !current.is_empty() {
                result.push(current.clone());
                current = word.to_string();
            } else {
                if !current.is_empty() { current.push(' '); }
                current.push_str(word);
            }
        }
        if !current.is_empty() { result.push(current); }
        result
    }

    fn render_status(&self, f: &mut ratatui::Frame, area: Rect) {
        let style = if self.streaming {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::DarkGray)
        };

        let status = Paragraph::new(self.status.as_str())
            .style(style)
            .block(Block::default()
                .borders(Borders::TOP)
                .style(Style::default().fg(Color::DarkGray)));
        f.render_widget(status, area);
    }

    fn render_input(&self, f: &mut ratatui::Frame, area: Rect) {
        let input_style = if self.streaming {
            Style::default().fg(Color::DarkGray).bg(Color::Rgb(30, 30, 30))
        } else {
            Style::default().fg(Color::White).bg(Color::Rgb(20, 20, 20))
        };

        let prefix = Span::styled(PROMPT_SYMBOL, Style::default().fg(Color::Green));
        let text = Span::styled(self.input.as_str(), input_style);
        let mut spans = vec![prefix, text];

        if !self.streaming {
            let _cursor_pos = PROMPT_SYMBOL.len() + self.cursor;
            spans.push(Span::styled("█", Style::default().fg(Color::White).bg(Color::White)));
        }

        let input = Paragraph::new(Line::from(spans))
            .block(Block::default()
                .borders(Borders::TOP)
                .title(" Input ")
                .style(Style::default().fg(Color::DarkGray)));
        f.render_widget(input, area);
    }
}
