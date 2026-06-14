# aibsd — FreeBSD-first AI coding agent

CLI AI agent для программирования и работы в среде FreeBSD. Написан на Rust.

## Возможности

- **FreeBSD-first дизайн**: глубокая интеграция с jails, ZFS, pf, pkg, портами
- **Несколько LLM провайдеров**: Anthropic Claude, OpenAI, Ollama (локальные модели)
- **Богатый набор инструментов**: файловые операции, bash, поиск, git
- **Режимы работы**: интерактивный TUI, однострочные запросы, HTTP сервер
- **MCP протокол**: расширение функциональности через Model Context Protocol (в планах)

## Быстрый старт

```bash
# Сборка
cargo build --release

# Запуск TUI
cargo run -- tui

# Однострочный запрос
cargo run -- run "покажи список jail"

# С указанием конфига
cargo run -- --config ~/.config/aibsd/config.toml tui
```

## Конфигурация

`~/.config/aibsd/config.toml`:

```toml
[llm]
provider = "ollama"        # ollama | anthropic | openai
model = "codellama:7b"
api_url = "http://localhost:11434"
max_tokens = 4096
temperature = 0.2
```

## Архитектура

```
aibsd/
├── src/
│   ├── main.rs          # Точка входа
│   ├── cli.rs           # CLI аргументы (clap)
│   ├── config.rs        # TOML конфигурация
│   ├── agent/           # Цикл агента (prompt -> LLM -> tools)
│   ├── llm/             # Провайдеры LLM (Anthropic, OpenAI, Ollama)
│   ├── tools/           # Инструменты (core + freebsd)
│   ├── session/         # SQLite хранилище сессий
│   └── tui/             # Терминальный интерфейс (ratatui)
```

## Лицензия

MIT
