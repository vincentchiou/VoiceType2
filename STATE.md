# VoiceType2 專案狀態與記憶檔案 (STATE.md)

> 本檔案為 OneStack 一人公司多 Agent 協同系統之單一真實狀態來源（Single Source of Truth），記錄專案架構決策、歷史教訓、執行狀態與交付驗收紀錄。

---

## 📌 基本資訊 (Meta)

- **專案名稱**：VoiceType2（AI 全域語音打字工具）
- **目前版本**：v1.0.4（旗艦正式版）
- **最後更新時間**：2026-11-27
- **專案模式 (mode)**：`maintenance_and_distribution`
- **當前階段 (phase)**：`ship_and_handoff_complete`
- **鎖定範圍 (scope_lock)**：`core_pipeline_stable` (F7~F12 全域鉤子、Whisper+LLaMA 雙層繁中校準、自訂記憶歷程)

---

## 🏛️ 架構決策紀錄 (Decisions)

1. **[ADR-001] 全域鍵盤監聽採用 Windows 原生 WH_KEYBOARD_LL**：
   - *決策理由*：第三方 hotkey crate 或 Tauri 前端事件 relay 會被 Windows 系統攔截或丟失。採用 Win32 底層鉤子直通後端 `crossbeam-channel`，焦點在任何程式均能 100% 穩定捕獲。
2. **[ADR-002] 雙層正體繁體中文保障機制**：
   - *決策理由*：單純仰賴後續轉繁容易產生簡繁語意偏誤。Whisper 端注入繁中提示詞引導音訊辨識，LLM 端強制鎖定台灣慣用語法並濾除簡體字，實現零簡體字外洩。
3. **[ADR-003] 長文智慧分段與條列分類排版**：
   - *決策理由*：使用者講述長篇工作報告或要點時，若輸出成無標點一長串文字閱讀不易。提示詞加入動態邏輯分段與 `1. 2. 3. / •` 條列規則；短句則維持單行精煉輸出。
4. **[ADR-004] 敏感金鑰防呆與實體發布包分離**：
   - *決策理由*：`voicetype-config.json` 包含使用者真實 Groq API Key，必須嚴格列入 `.gitignore`，且發布包腳本 `package-release.ps1` 實作主動攔截阻擋，防止任何個人金鑰打包外洩。
5. **[ADR-005] 動態虛擬按鍵映射 (F7~F12)**：
   - *決策理由*：筆電使用者常因 F9 綁定螢幕亮度或靜音而衝突，改用 `AtomicU32` 動態更新虛擬鍵碼，設定後免重啟即刻切換生效。

---

## 🧠 經驗與踩坑教訓 (Lessons & Gotchas)

1. **中文路徑編譯坑**：
   - Node/Cargo 於含中文雲端硬碟路徑（如 `H:\我的雲端硬碟\...`）執行建置時易發生神秘失敗，原始碼建置必須放在純英文路徑 `C:\temp\VoiceType2\`。
2. **WebView2 依賴與安裝包策略**：
   - Windows 11 內建 WebView2，直接使用綠色免安裝版最快；Windows 10 早期或精簡版若缺 WebView2 會閃退，必須指引使用者使用 NSIS 安裝版（自動下載 Bootstrapper）。
3. **剪貼簿焦點延遲**：
   - 模擬 `Ctrl+V` 貼上時需短暫暫停 200~300ms，以防剪貼簿寫入尚未完成即觸發貼上。

---

## 👥 OneStack 團隊執行追蹤 (team_execution)

```yaml
team_execution:
  goal: "VoiceType2 發布包全面體檢、清除金鑰洩漏風險、實作繁中長文排版、自訂快捷鍵與歷史紀錄，完成發布"
  tasks:
    - id: "T1-Security"
      role: "onestack-guard"
      status: "done_verified"
      files: ["voicetype-config.json", "package-release.ps1"]
      validation: "成功自發布包移除真實 Key，打包腳本驗證主動攔截生效"
    - id: "T2-CoreEnhancement"
      role: "onestack-build"
      status: "done_verified"
      files: ["src-tauri/src/api.rs", "src-tauri/src/hotkey.rs", "src-tauri/src/lib.rs", "src/App.tsx"]
      validation: "前後端編譯通過，實測 F7~F12 動態切換、繁中提示詞與歷史歷程面板"
    - id: "T3-Documentation"
      role: "onestack-ship"
      status: "done_verified"
      files: ["VoiceType2-圖文使用手冊.docx", "VoiceType2-圖文使用手冊.html", "使用說明.md", "README.md", "SOP-操作手冊.md"]
      validation: "Word 與 HTML 圖文手冊產出完備，排版美觀"
    - id: "T4-Release"
      role: "onestack-ship"
      status: "done_verified"
      files: ["package-release.ps1", "SHA256.txt", "VoiceType2-v1.0.4-發布給親友.zip"]
      validation: "GitHub Release v1.0.4 成功發布並上傳乾淨資產"
  decisions:
    - "ADR-001 ~ ADR-005 已全面落實"
  blockers: []
  next: "持續收集使用者回饋；下一階段規劃系統列背景最小化常駐圖示 (System Tray)"
```
