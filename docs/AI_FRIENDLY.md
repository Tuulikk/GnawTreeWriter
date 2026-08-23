# GnawTreeWriter AI-Friendly Features

## Quick Reference

### Token-Aware Analysis
```bash
gnawtreewriter analyze src/main.rs --format summary
# Shows estimated tokens per file with context window warnings
```

### Code Compression
```bash
gnawtreewriter compress src/main.rs --stats
# Replaces function bodies with ⋮---- placeholders (~70% reduction)
```

### Project Packing
```bash
gnawtreewriter pack . --compress --format markdown
gnawtreewriter pack src/ --include rs,py --instructions "Focus on auth"
# Packages entire project with token counts and optional compression
```

### Intelligent Context Curation
```bash
gnawtreewriter curate "authentication login" --max-tokens 5000
gnawtreewriter curate "database" --strategy recent
# Selects only relevant files instead of dumping everything
```

### Secret Detection
Secrets are auto-detected and redacted in pack output.
Supports: AWS keys, GitHub tokens, JWT, private keys, Stripe, and more.

## MCP Tools

All features available as MCP tools:
- `compress` - Compress a file
- `pack` - Package entire project
- `curate` - Intelligent file selection

## Integration with AI Agents

### Claude Code
```json
{
  "mcpServers": {
    "gnawtreewriter": {
      "command": "gnawtreewriter",
      "args": ["mcp", "stdio"]
    }
  }
}
```

### Usage in Prompts
```
Use gnawtreewriter to curate context for this task:
gnawtreewriter curate "implement user authentication" --max-tokens 4000

Then analyze the curated files:
gnawtreewriter analyze <file> --format summary
```
