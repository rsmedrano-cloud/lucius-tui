# Lucius TUI

A terminal user interface for interacting with Ollama models, built with Rust.

## Features

- Real-time streaming of responses from Ollama.
- Interactive chat interface.
- Settings screen to configure the Ollama URL and select models.
- Markdown rendering of chat history.
- Mouse and keyboard scrolling of chat history.
- Interruptible responses with the `Esc` key.

## How to Run

1. Make sure you have an Ollama instance running.  
2. Clone the repository.  
3. Run the application with `cargo run`.

## Feel the Vibe

Lucius TUI walks into a bar and orders an Ollama model.  
The bartender asks:  
> **“Do you want it *streaming* or *on the rocks*?”**

Lucius replies:  
> **“Streaming, please. I like my responses flowing… kind of like my segfaults when I forget to run `cargo check`.”**

The bartender continues:  
> **“And how should I serve it?”**

Lucius winks:  
> **“With Markdown, of course. I like taking notes on the go… even if they won't stop scrolling afterwards.”**

Finally, the bartender asks:  
> **“Need to interrupt at any moment?”**

Lucius smirks:  
> **“Yes. If things get too heavy, I’ll just hit ESC… the same way I escape from my responsibilities.”**

Everyone at the bar laughs…  
except the main thread — still waiting for the lock.
