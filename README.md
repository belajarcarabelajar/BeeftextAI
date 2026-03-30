# BeefText AI ⚡

BeefText AI is a modern, cross-platform text expansion utility built as a conceptual and spiritual successor to [Beeftext](https://github.com/xmichelo/Beeftext). It supercharges your daily typing workflows by combining instant local text expansions with the power of localized AI code generation, bringing text automation into the AI era.

Built from scratch using **Tauri 2.x**, **Rust**, and **React**, BeefText AI is designed for blistering performance, low footprint, and high modularity. 

*(A high-performance dark-theme UI with instant expansion feedback)*

---

## 🎯 Features

- **Global Text Expansion (Background)**: Type a shortcut trigger anywhere (e.g., in Chrome, VSCode, Terminal) and it instantly expands to your predefined snippet. Powered by `rdev`, watching in the background independently of the OS UI.
- **AI Text Generation `#{ai:prompt}`**: Call an LLM natively during text generation. Write `#{ai:write a professional rejection letter}` and your snippet expands dynamically through **Ollama**.
- **Template Variables**: Supercharge snippets with built-in variables:
  - `#{clipboard}`: Paste clipboard content inside an expansion.
  - `#{date}` / `#{time}` / `#{dateTime:format}`: Print dynamic current timestamps.
  - `#{combo:keyword}`: Chain snippets together.
  - `#{envVar:name}`: Read environment variables actively.
  - Text modifiers: `#{upper:text}`, `#{lower:text}`, `#{trim:text}`
- **OmniSearch & Semantic Ranking**: Searching snippets isn't limited to keywords. BeefText AI generates local embeddings per snippet (via `nomic-embed-text`) allowing you to retrieve snippets based on context and meaning, not just exact words.
- **Integrated AI Chatbot**: Talk to the local AI to brainstorm or convert ideas into text snippets automatically with a single click.
- **Strict & Loose Matching**: Control how keywords are triggered contextually to avoid accidental expansions.
- **Desktop Notifications**: Instant "toast" notifications on successful snippet insertion.
- **Migration & Backup**: Import legacy `.json` backups of your old Beeftext v10 app easily. Full internal system backup mechanism provided.

## 🏗️ Architecture Stack

- **Backend**: Rust 1.94+
- **Frontend**: React, TypeScript, Vite
- **Webview Framework**: Tauri 2.x
- **Data Persistence**: SQLite (via `rusqlite`)
- **Local AI Provider**: Ollama REST API (`nemotron-3-super:cloud` and `nomic-embed-text`)
- **Keyboard Global Hook**: `rdev`
- **Clipboard Management**: `arboard`

## 🚀 Getting Started

### Prerequisites
1. **Node.js** v20+ & **npm**
2. **Rust Environment** (`rustup default stable`)
3. **[Ollama](https://ollama.com/)** running locally (`localhost:11434`)
    - Ensure models are pulled:
      - `ollama pull nemotron-3-super:cloud` (or whatever base text model you prefer)
      - `ollama pull nomic-embed-text`

### Installation

Clone the repository and install frontend dependencies:

```bash
git clone https://github.com/belajarcarabelajar/BeeftextAI.git
cd BeeftextAI
npm install
```

### Running Locally (Dev Mode)

To start the development server (Frontend + Rust Backend hot reload):

```bash
npm run tauri dev
```

### Production Build

For a completely standalone Windows `.msi` / `.exe` bundle, run:

```bash
npm run tauri build
```
*(Ensure all MSVC build tools are properly configured if on Windows.)*

## ⚙️ Configuration

1. Launch the app. You'll see the global system tray icon indicating that the keyboard hook is active.
2. Go to **Settings (⚙️)** in the sidebar.
3. Verify your **Ollama Server** (default `http://localhost:11434`).
4. Set your preferred Base LLM and Embedding Model names.
5. Setup Interface Language (Supports EN/ID).

## 🗂 Data Privacy

All text expansions, semantic vector embeddings, and AI chat logs remain strictly **on-device**. Nothing is sent to the cloud. BeefText AI talks exclusively to your local Ollama daemon or local SQLite database.

## 🤝 Contributing

Contributions to BeefText AI are strictly welcomed! If you have suggestions or fixes for the global hook stability on specific Linux window managers or Mac architectures, feel free to open a PR.

## 📄 License

This project is licensed under the MIT License. It was heavily inspired by the original C++/Qt project *[Beeftext](https://github.com/xmichelo/Beeftext)* by Xavier Michelon.
