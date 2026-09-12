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
            .text("language", "zh".to_string());

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
            "你是一個專業的中文語音轉文字校正助手。\n\
             {}請校正以下這句語音轉文字的內容：\n\
             1. 修正錯別字與同音字（可參考前文判斷正確用字，例如「再/在」「的/得/地」「他/她/它」）\n\
             2. 加上適當的標點符號\n\
             3. 移除贅字和語氣詞（如：那個、就是、然後、嗯、啊、呃）\n\
             4. 保持原意，不要改變內容，不要續寫，不要加評論\n\
             5. 只輸出校正後的本句文字\n\n\
             本句原文：{}",
            context_block, text
        );

        let request = ChatRequest {
            model: "llama-3.1-8b-instant".to_string(),
            messages: vec![ChatMessage { role: "user".to_string(), content: prompt }],
            temperature: 0.3,
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
