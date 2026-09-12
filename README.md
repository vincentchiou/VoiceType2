# VoiceType2 — AI 語音輸入工具（取代 Windows H）

> 按住 **F9** 說話 → 鬆開 → 自動轉錄＋AI 校正 → 文字自動貼到游標位置。
> 在任何 Windows 程式都能用（記事本、Word、瀏覽器、LINE…）。

---

## 1. 快速開始（邱老師專用）

### 新電腦安裝（二選一）

| 方式 | 檔案 | 說明 |
|------|------|------|
| **A. 安裝版（推薦）** | `VoiceType_1.0.1_x64-setup.exe` | 雙擊安裝；若缺 WebView2 會自動下載安裝，全新 Win10／Win11 都可用 |
| B. 免安裝版 | `voice-type.exe` | 直接執行；需該電腦已有 WebView2（Win10 1803＋／Win11 內建通常有） |

兩種方式**都需要**：麥克風＋網路＋Groq API Key（免費申請：https://console.groq.com）。

| 步驟 | 動作 |
|------|------|
| 1 | 執行安裝版或免安裝版 |
| 2 | 首次執行輸入 **Groq API Key** → 按「儲存」 |
| 3 | 開啟任何程式，把游標放到要輸入的位置 |
| 4 | **按住 F9** 開始錄音 |
| 5 | 說話 |
| 6 | **鬆開 F9**，等 2–3 秒 |
| 7 | 校正後的文字自動出現在游標位置 ✅ |

也可以直接點視窗上的 **🎤 按鈕** 開始／停止錄音（功能相同）。

### 設定檔

`voicetype-config.json`（與 exe 同目錄，自動產生）：

```json
{
  "groq_api_key": "gsk_...",
  "auto_correct": true
}
```

> ⚠️ 此檔含 API Key，**不要外流、不要上傳 GitHub**。

---

## 2. 功能特色

- 🎤 **全域語音輸入**：F9 按住錄音、鬆開停止，系統層級 keyboard hook（`SetWindowsHookExW`），焦點在任何程式都能觸發
- 🗣️ **語音轉文字**：Groq Whisper API（`whisper-large-v3-turbo`），免費、免下載模型、中文辨識佳
- 🧠 **AI 文字校正**：Groq LLM（`llama-3.1-8b-instant`）自動修正錯別字／同音字、加標點、移除贅字（那個、就是、然後、嗯、啊）；**一律輸出繁體中文**；科技／AI 術語優先（API、LLM、GitHub、Whisper、prompt、token 等保留英文原文不翻譯）
- 🔗 **上下文記憶校正**：自動記住最近 **5 句**已校正內容，一起送給 LLM 參考 → 同音字判斷更準（再／在、的／得、他／她）、語意連貫（關程式即清空，換主題重開即可）
- 📋 **自動貼上**：校正後自動寫入剪貼簿並模擬 `Ctrl+V` 貼到游標位置
- 🪟 **極簡視窗**：400px 設定＋狀態視窗，不占畫面

---

## 3. 技術架構

- **前端**：React 19 + TypeScript + Vite + Tailwind（僅做設定頁＋狀態顯示＋按鈕錄音）
- **後端**：Rust + Tauri 2.x
- **錄音**：`cpal`（預設麥克風）+ `hound`（寫 WAV 到 `%TEMP%\voicetype\`）
- **快捷鍵**：Windows 原生 `WH_KEYBOARD_LL` hook（經 `winapi`），不依賴第三方 hotkey crate
- **轉錄／校正**：`reqwest` blocking client → Groq OpenAI 相容 API
- **執行緒模型**：
  ```
  F9 按下 → hook 線程 → crossbeam-channel → 管線線程 → 錄音／轉錄／校正／剪貼簿／Ctrl+V
  ```
  全程在 Rust 後端完成，**不經過 Tauri 事件 relay**（事件 relay 曾經完全收不到，詳見 §6）。

### 參考專案

架構靈感來自 [cablate/GPT-Typeless](https://github.com/cablate/GPT-Typeless)
（按住錄音→鬆開轉錄→複製到剪貼簿）。差異：本專案改用 **Groq 免費 API**（免 ChatGPT Token），並加上 **LLM 校正＋上下文記憶＋自動貼上**。

### 檔案結構（建置原始碼）

```
C:\temp\VoiceType2\            ← 建置用（全英文路徑，避開中文路徑坑）
├── src\App.tsx                ← 設定頁＋狀態＋按鈕錄音
├── src-tauri\src\
│   ├── lib.rs                 ← 命令＋F9 管線線程＋上下文歷史（HISTORY）
│   ├── audio.rs               ← cpal 錄音＋hound 寫 WAV
│   ├── api.rs                 ← Groq Whisper 轉錄＋LLM 校正（含上下文版）
│   └── hotkey.rs              ← F9 hook＋Ctrl+V 模擬貼上
└── dist\                      ← npm run build 產物（tauri build 會內嵌）

H:\我的雲端硬碟\Agent\project\VoiceType2\   ← 交付目錄
├── VoiceType_1.0.1_x64-setup.exe  ← 安裝版（缺 WebView2 自動下載）
├── voice-type.exe             ← 免安裝版（約 11MB，前端已內嵌）
├── voicetype-config.json      ← 設定（含 API Key，勿外流）
└── README.md                  ← 本檔
```

---

## 4. 重新建置（開發者）

```powershell
$env:PATH = "C:\Users\user\.cargo\bin;$env:PATH"
cd "C:\temp\VoiceType2"
npx tauri build        # ← 一定要用這個，不要只用 cargo build
```

> ⚠️ **一定要用 `npx tauri build`**：只跑 `cargo build --release` 不會打包前端，
> exe 會顯示 `localhost 拒絕連線`（ERR_CONNECTION_REFUSED）。
> 建好後把 `src-tauri\target\release\voice-type.exe` 複製到交付目錄。
>
> 附帶問題：`tauri build` 最後的 NSIS 步驟會因 `Couldn't find a .ico icon`
> 失敗，但 exe 本體已正確產出，無需理會。

依賴：Node.js 18+、Rust（`C:\Users\user\.cargo\bin`）、Tauri CLI。

---

## 5. 版本紀錄

| 日期 | 版本內容 |
|------|----------|
| 2026-09-12 | v1：基於 GPT-Typeless 架構＋Groq API，`Ctrl+Win` 快捷鍵（被 Windows 攔截，不可用） |
| 2026-09-12 | v2：改 `Ctrl+Shift+R`，修視窗缺失（`tauri.conf.json` windows 曾為空）、修前端重複函式 |
| 2026-09-12 | v3：改 `global-hotkey` crate → 無反應；改 `WH_KEYBOARD_LL` hook＋Tauri 事件 relay → 仍無反應 |
| 2026-09-12 | v4：hook→channel→管線線程**全後端直連**，F9 可用；加入**上下文記憶校正（前 5 句）**；v1.0.0 發布 |
| 2026-09-12 | v1.0.1（現行）：校正提示詞改為**科技／AI 術語優先＋一律繁體中文** |

---

## 6. 已知限制／踩坑紀錄

1. **快捷鍵固定為 F9**：`Ctrl+Win` 會被 Windows 系統攔截；`global-hotkey` crate 在此專案完全收不到事件；`WH_KEYBOARD_LL` hook＋Tauri `emit/listen` relay 也收不到。最終解法是 hook→`crossbeam-channel`→管線線程全在後端直連。
2. **貼上依賴 `Ctrl+V` 模擬**：目標程式必須有文字焦點；剪貼簿會被覆寫。
3. **中文路徑坑**：`H:\我的雲端硬碟\...` 下 `npm install` 會失敗，建置必須在 `C:\temp\VoiceType2`。
4. **NSIS 打包**：`bundle.icon` 需指向 `icons/icon.ico`（`icon.png` 會報 `Couldn't find a .ico icon`）；`webviewInstallMode` 在 Tauri 2.11 的 serde 格式是 `{"type": "downloadBootstrapper"}` 物件形式，但**預設值已是 DownloadBootstrapper**，直接省略該欄即可。
5. **舊版 VoiceType（`project\VoiceType\`）**：Whisper 為佔位符（假字串）、無全域快捷鍵，已被本專案取代，勿混用。

---

## 7. 下一步（待辦）

- [ ] 自訂快捷鍵（目前寫死 F9）
- [ ] 系統列圖示＋開機啟動
- [ ] 上下文句數可調、歷史紀錄顯示
- [ ] 錄音指示浮窗（錄音中提示）
- [ ] 正式版 icon＋NSIS 安裝程式

---

**最後更新**：2026-09-12（v1.0.1）
**狀態**：✅ 可用（F9 全域錄音→轉錄→上下文校正→自動貼上）
**GitHub**：https://github.com/vincentchiou/VoiceType2（v1.0.1 Release）
