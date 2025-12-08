# Instrucciones de Uso y Desarrollo — Lucius TUI

Este archivo resume las instrucciones generadas tras analizar el repositorio `lucius-tui`.

---

## 🔎 Resumen del repo
- Proyecto: `lucius-tui` (Rust) — TUI para LLMs (Ollama) con soporte para tool-call via MCP.
- Binarios: `lucius` (principal), `shell-mcp` (MCP server).
- Módulos clave:
  - `src/main.rs` — UI, flujo LLM, loops y gestión general.
  - `src/mcp.rs` — Cliente MCP (lanza `shell-mcp` y hace llamadas JSON-RPC por stdin/stdout).
  - `src/bin/shell-mcp.rs` — Servidor JSON-RPC (exec, remote_exec).
  - `src/config.rs` — Persistencia de config (`~/.config/lucius/lucius_config.toml`).
  - `src/context.rs` — Búsqueda y creación automática de `LUCIUS.md`.

---

## 🚀 Quick Start — build & run
### Requisitos
- Rust >= 1.70.0 (recomendado usar rustup).
- Ollama u otro servidor compatible ejecutándose y accesible por URL.
- Opcional: `wl-copy` si usas Wayland para la funcionalidad del portapapeles (`Ctrl+Y`).

### Compilar
```bash
cargo build
```

### Build (release)
```bash
cargo build --release
```

### Ejecutar (desde cargo)
```bash
cargo run --bin lucius
```

### Ejecutar `shell-mcp` (opcional, en otro terminal para depuración)
```bash
cargo run --bin shell-mcp
# o
./target/debug/shell-mcp
```

> Nota: `main.rs` intenta lanzar `target/debug/shell-mcp` por defecto. Si corres `lucius` en release, lanza manualmente `shell-mcp` o define ruta.

---

## ⚙️ Configuración inicial (UI)
1. Ejecuta `lucius`.
2. Presiona `Ctrl+S` para abrir Settings.
3. Modifica `Ollama URL` (por defecto `http://192.168.1.42:11434`).
4. Presiona `Ctrl+R` para refrescar la lista de modelos (llamada a `/api/tags`).
5. Selecciona un modelo, presiona `Enter` para configuración final, y `Esc` para volver al chat.

Las configuraciones se guardan en: `~/.config/lucius/lucius_config.toml`.

---

## 🧪 Interacción LLM y Tool-calls
- El LLM puede emitir tool calls usando el formato:

```
[TOOL_CALL] {"tool":"exec", "params":{"command":"uptime"}} [END_TOOL_CALL]
```

- Flujo:
  1. App detecta el `ToolCall` con `mcp::parse_tool_call`.
  2. Se manda la solicitud a `mcp::McpClient`.
  3. `shell-mcp` ejecuta la acción (exec, remote_exec) y devuelve un JSON en stdout.
  4. Resultado se añade a la conversación como `Tool Result:` y el LLM recibe el contexto actualizado.

---

## 🔧 Depuración y logs
- Archivo de logs: `lucius.log` creado en el directorio de ejecución.
- Para logs detallados:
```bash
RUST_LOG=debug RUST_BACKTRACE=1 cargo run --bin lucius
```
- Si `MCP client not running` aparece, asegúrate de que `shell-mcp` exist a en `target/debug` o ejecútalo manualmente.
- `Ctrl+Y` usa `wl-copy` (Wayland). Instala `wl-clipboard` si es necesario.

---

## 🧭 Developer notes (observaciones técnicas)
- `mcp_server_name` está codificado como `target/debug/shell-mcp`. Mejorar a través de variable de entorno o argumento CLI:
```
LUCIUS_MCP_SERVER
```
 - `McpClient::call` ahora es asíncrono (`async`) y utiliza `tokio::task::spawn_blocking` para evitar bloquear los hilos de tokio. Se introdujo un `Arc<Mutex<Child>>` para la comunicación con el proceso MCP.
  - Garantizar que `shell-mcp` devuelve una única línea JSON por petición.
 - `parse_tool_call` usa `Regex` para extraer JSON; se añadieron tests unitarios para validar parseo correcto y casos con JSON inválido.
- `context::load_lucius_context` crea `LUCIUS.md` en el CWD si no existe.
 - `context::load_lucius_context` crea `LUCIUS.md` en el CWD si no existe; además se añadió `load_lucius_context_from(start_path)` para permitir búsquedas sin mutar el CWD (útil en tests).

- Selección y copia en la zona de conversación: ahora puedes usar el mouse para seleccionar líneas en el área de conversación (clic y arrastrar) y presionar `Ctrl+Y` para copiar la selección al portapapeles (usa `wl-copy` en Wayland). Si no hay selección, `Ctrl+Y` copia la última respuesta del asistente como antes.
- Scroll con el mouse: la rueda del ratón hace scroll de la conversación (ya implementado).

---

## ✅ Pruebas sugeridas y CI
- No hay tests por defecto. Añadir:
  - Tests unitarios para `parse_tool_call`.
  - Tests para `config::load` y `save` (usar un directorio temporal).
  - Tests para `context::load_lucius_context`.

- Configurar CI (GitHub Actions):
  - `cargo test --all`.
  - `cargo fmt -- --check`.
  - `cargo clippy -- -D warnings`.

---

## 🛠️ Cómo añadir una nueva herramienta (MCP)
1. Añadir handler en `src/bin/shell-mcp.rs` y mapear el método en `match`.
2. Retornar `JsonRpcResponse` válido (result o error).
3. Si la herramienta puede durar mucho tiempo, considerar streaming o chunking para no bloquear.

---

## 🧠 Mejoras propuestas (prioridad alta → baja)
1. Soporte para configurar `mcp_server_name` via env var o CLI param.
2. Añadir tests unitarios y de integración.
3. Confirmación del usuario antes de ejecutar `exec` o `remote_exec`.
4. Mejor manejo de binarios `shell-mcp` en release vs debug.
5. Hacer `McpClient` asíncrono o un hilo dedicado para llamadas blocking.
6. Leer salida hasta un JSON completo en vez de solo 1 línea, o asegurar `shell-mcp` imprime 1 JSON por request.
7. Documentar riesgos de seguridad cuando el LLM ejecuta comandos arbitrarios.

---

## 🧾 Contribución rápida
1. Clona y compila:
```bash
git clone https://github.com/rsmedrano-cloud/lucius-tui.git
cd lucius-tui
cargo build
```
2. Corre `shell-mcp` en otro terminal (opcional):
```bash
cargo run --bin shell-mcp
```
3. Ejecuta la UI:
```bash
cargo run --bin lucius
```
4. Formatea y checa lint:
```bash
cargo fmt
cargo clippy
```

---

## ⚠️ Notas finales / gotchas
- `mcp_server_name` por defecto está en `target/debug`. Adaptarlo para release o definir variable/CLI.
- `Ctrl+Y` necesita `wl-copy` en Wayland.
- `LUCIUS.md` se crea en `cwd` si no existe; revisa su contenido.

---

¿Quieres que además haga una PR con alguna mejora concreta (p. ej. env var para `mcp_server_name`, tests unitarios para `parse_tool_call`, o GitHub Actions para CI)?
