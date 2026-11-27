// VoiceType - Simple API module
use reqwest::blocking::multipart;
use serde::Deserialize;

pub struct GroqClient {
    api_key: String,
    client: reqwest::blocking::Client,
}

#[derive(Deserialize)]
struct WhisperResponse {
    text: String,
}

#[derive(serde::Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<ChatMessage>,
    temperature: f32,
}

#[derive(serde::Serialize, serde::Deserialize)]
struct ChatMessage {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<ChatChoice>,
}

#[derive(Deserialize)]
struct ChatChoice {
    message: ChatMessage,
}

impl GroqClient {
    pub fn new(api_key: String) -> Self {
        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(60))
            .build()
            .expect("Cannot create HTTP client");
        Self { api_key, client }
    }

    pub fn transcribe(&self, audio_path: &std::path::Path) -> Result<String, String> {
        eprintln!("[API] Transcribing: {}", audio_path.display());
        let data = std::fs::read(audio_path).map_err(|e| format!("Read file failed: {}", e))?;
        let name = audio_path.file_name().and_then(|n| n.to_str()).unwrap_or("audio.wav");

        let part = multipart::Part::bytes(data).file_name(name.to_string());
        let form = multipart::Form::new()
            .part("file", part)
            .text("model", "whisper-large-v3-turbo".to_string())
            .text("language", "zh".to_string())
            .text("prompt", "以下是繁體中文的日常生活、工作記錄、教學筆記與科技對話。".to_string());

        let resp = self.client
            .post("https://api.groq.com/openai/v1/audio/transcriptions")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .multipart(form)
            .send()
            .map_err(|e| format!("API request failed: {}", e))?;

        let status = resp.status();
        let body = resp.text().unwrap_or_default();
        eprintln!("[API] Whisper response status: {}, body: {}", status, &body[..body.len().min(200)]);

        if !status.is_success() {
            return Err(format!("API error ({}): {}", status, body));
        }

        let r: WhisperResponse = serde_json::from_str(&body)
            .map_err(|e| format!("JSON parse error: {}", e))?;
        Ok(r.text)
    }

    pub fn correct_text(&self, text: &str) -> Result<String, String> {
        self.correct_with_context(text, &[])
    }

    /// Correct text using conversation context (previous sentences).
    /// `context` = list of previous corrected sentences, oldest first.
    pub fn correct_with_context(&self, text: &str, context: &[String]) -> Result<String, String> {
        eprintln!("[API] Correcting: {}", text);

        let context_block = if context.is_empty() {
            String::new()
        } else {
            format!("前文（已校正過的內容，供參考以維持語意連貫）：\n{}\n\n", context.join("\n"))
        };

        let prompt = format!(
            "你是一個專業的繁體中文語音轉文字校正與智慧整理專家，熟悉台灣習慣用語與科技 AI 領域術語。\n\
             {}請校正並整理以下語音轉文字內容：\n\
             【核心原則】\n\
             1. 絕對輸出繁體中文（正體中文，台灣習慣用語法），原文若有任何簡體字一律轉換為繁體，嚴禁輸出任何簡體字。\n\
             2. 科技與 AI 專有名詞、縮寫、英文術語保留原文不翻譯（例如：API、LLM、AI、GitHub、Whisper、prompt、token、Tauri、ChatGPT、Groq、Ollama、agent、model、Python、Rust 等），並修正大小寫與拼寫。\n\
             3. 修正錯別字與同音字（參考前文判斷正確用字，如「再/在」「的/得/地」「他/她/它」「做/作」）。\n\
             4. 移除口語贅字與無意義語氣詞（如：那個、就是說、然後、嗯、啊、呃、的話）。\n\
             5. 加上正確適當的標點符號。\n\
             【長文智慧段落與整理分類】\n\
             6. 若說話內容較長、包含多個重點、流程步驟或列舉事項時，請主動進行「邏輯分段」與「分類條列」（適當使用換行或 1. 2. 3. / • 標記），讓長篇內容結構層次分明、易於閱讀。\n\
             7. 若只是簡短的一兩句話或日常口語輸入，請保持精煉自然，不需強行條列。\n\
             【嚴格限制】\n\
             8. 保持發言原意，不捏造、不增減實質內容、不續寫、不加任何個人評論。\n\
             9. 絕對只輸出校正與排版後的最終文字成果，嚴禁輸出任何前言贅語（如「以下是校正結果：」）、問候語或後記說明。\n\n\
             待校正原文：\n{}",
            context_block, text
        );

        let request = ChatRequest {
            model: "llama-3.1-8b-instant".to_string(),
            messages: vec![ChatMessage { role: "user".to_string(), content: prompt }],
            temperature: 0.2,
        };

        let resp = self.client
            .post("https://api.groq.com/openai/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .map_err(|e| format!("LLM request failed: {}", e))?;

        let status = resp.status();
        let body = resp.text().unwrap_or_default();
        eprintln!("[API] LLM response status: {}", status);

        if !status.is_success() {
            eprintln!("[API] LLM error: {}", body);
            return Ok(text.to_string());
        }

        let r: ChatResponse = serde_json::from_str(&body)
            .map_err(|e| format!("LLM JSON error: {}", e))?;

        Ok(r.choices.first().map(|c| c.message.content.trim().to_string()).unwrap_or_else(|| text.to_string()))
    }
}
