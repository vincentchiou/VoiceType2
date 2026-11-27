import { useState, useEffect, useCallback, useRef } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'

interface AppConfig {
  groq_api_key: string
  auto_correct: boolean
  auto_start: boolean
  hotkey: string
  context_sentences: number
}

function App() {
  const [config, setConfig] = useState<AppConfig>({
    groq_api_key: '',
    auto_correct: true,
    auto_start: false,
    hotkey: 'F9',
    context_sentences: 5,
  })
  const [isRecording, setIsRecording] = useState(false)
  const [status, setStatus] = useState('載入中...')
  const [showSettings, setShowSettings] = useState(false)
  const [showHistory, setShowHistory] = useState(false)
  const [historyList, setHistoryList] = useState<string[]>([])

  const [apiKeyInput, setApiKeyInput] = useState('')
  const [autoStartInput, setAutoStartInput] = useState(false)
  const [hotkeyInput, setHotkeyInput] = useState('F9')
  const [contextSentencesInput, setContextSentencesInput] = useState(5)

  const isRecordingRef = useRef(false)
  const processingRef = useRef(false)

  const refreshHistory = useCallback(async () => {
    try {
      const list = await invoke<string[]>('get_history_command')
      setHistoryList(list || [])
    } catch {
      setHistoryList([])
    }
  }, [])

  const startRec = useCallback(async () => {
    if (isRecordingRef.current || processingRef.current) return
    try {
      await invoke('start_recording')
      isRecordingRef.current = true
      setIsRecording(true)
      setStatus('錄音中...')
    } catch (err) {
      setStatus(`錯誤: ${err}`)
    }
  }, [])

  const stopRec = useCallback(async () => {
    if (!isRecordingRef.current || processingRef.current) return
    try {
      processingRef.current = true
      setIsRecording(false)
      isRecordingRef.current = false
      setStatus('處理中...')
      const result = await invoke<{ audio_path: string; duration: number }>('stop_recording')
      setStatus(`轉錄中 (${result.duration}秒)...`)
      const text = await invoke<string>('transcribe', { audioPath: result.audio_path })
      setStatus(`完成！已貼上: ${text.substring(0, 30)}...`)
      refreshHistory()
    } catch (err) {
      setStatus(`錯誤: ${err}`)
    } finally {
      processingRef.current = false
    }
  }, [refreshHistory])

  useEffect(() => {
    invoke<AppConfig>('get_config_command').then((cfg) => {
      const activeCfg = {
        groq_api_key: cfg.groq_api_key || '',
        auto_correct: cfg.auto_correct ?? true,
        auto_start: cfg.auto_start ?? false,
        hotkey: cfg.hotkey || 'F9',
        context_sentences: cfg.context_sentences || 5,
      }
      setConfig(activeCfg)
      setApiKeyInput(activeCfg.groq_api_key)
      setAutoStartInput(activeCfg.auto_start)
      setHotkeyInput(activeCfg.hotkey)
      setContextSentencesInput(activeCfg.context_sentences)

      // 以登錄檔實際狀態為準
      invoke<boolean>('get_autostart').then(setAutoStartInput).catch(() => {})
      if (!activeCfg.groq_api_key) {
        setShowSettings(true)
        setStatus('請先設定 API Key')
      } else {
        setStatus(`就緒 - 按 ${activeCfg.hotkey} 開始錄音`)
      }
      refreshHistory()
    }).catch((err) => {
      setStatus(`錯誤: ${err}`)
      setShowSettings(true)
    })

    // Listen for hotkey events (press/release)
    const p1 = listen<boolean>('recording-state', (event) => {
      if (event.payload) {
        isRecordingRef.current = true
        setIsRecording(true)
        setStatus('錄音中...')
      } else {
        isRecordingRef.current = false
        setIsRecording(false)
        setStatus('處理中...')
      }
    })

    // Listen for status updates from backend pipeline
    const p2 = listen<string>('status', (event) => {
      setStatus(event.payload)
      if (event.payload.startsWith('完成')) {
        isRecordingRef.current = false
        processingRef.current = false
        setIsRecording(false)
        refreshHistory()
      }
      if (event.payload.startsWith('錯誤')) {
        isRecordingRef.current = false
        processingRef.current = false
        setIsRecording(false)
      }
    })

    // Also support button clicks via frontend
    const p3 = listen<boolean>('hotkey-event', (event) => {
      if (event.payload) startRec()
      else stopRec()
    })

    return () => { p1.then(fn => fn()); p2.then(fn => fn()); p3.then(fn => fn()) }
  }, [startRec, stopRec, refreshHistory])

  const handleButton = async () => {
    if (isRecordingRef.current) await stopRec()
    else await startRec()
  }

  const saveSettings = async () => {
    try {
      const newConfig: AppConfig = {
        groq_api_key: apiKeyInput.trim(),
        auto_correct: config.auto_correct,
        auto_start: autoStartInput,
        hotkey: hotkeyInput,
        context_sentences: Number(contextSentencesInput) || 5,
      }
      await invoke('set_config', { config: newConfig })
      setConfig(newConfig)
      setShowSettings(false)
      setStatus(`就緒 - 按 ${newConfig.hotkey} 開始錄音`)
    } catch (err) {
      setStatus(`儲存失敗: ${err}`)
    }
  }

  const clearHistory = async () => {
    try {
      await invoke('clear_history_command')
      setHistoryList([])
    } catch (err) {
      console.error(err)
    }
  }

  // 畫面 1：設定介面
  if (showSettings) {
    return (
      <div style={{ padding: '16px 20px', maxWidth: '380px', margin: '0 auto', fontSize: '13.5px' }}>
        <h2 style={{ color: '#7c3aed', textAlign: 'center', margin: '0 0 16px 0', fontSize: '20px' }}>VoiceType 設定</h2>
        
        <div style={{ marginBottom: '14px' }}>
          <label style={{ display: 'block', marginBottom: '4px', fontWeight: 'bold' }}>Groq API Key</label>
          <input
            type="password"
            value={apiKeyInput}
            onChange={(e) => setApiKeyInput(e.target.value)}
            style={{ width: '100%', padding: '8px 10px', border: '1px solid #ccc', borderRadius: '6px', boxSizing: 'border-box' }}
            placeholder="gsk_..."
          />
          <p style={{ fontSize: '11px', color: '#666', marginTop: '3px' }}>免費申請: https://console.groq.com</p>
        </div>

        <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '10px', marginBottom: '14px' }}>
          <div>
            <label style={{ display: 'block', marginBottom: '4px', fontWeight: 'bold' }}>自訂快捷鍵</label>
            <select
              value={hotkeyInput}
              onChange={(e) => setHotkeyInput(e.target.value)}
              style={{ width: '100%', padding: '8px', border: '1px solid #ccc', borderRadius: '6px' }}
            >
              <option value="F9">F9（預設推薦）</option>
              <option value="F8">F8</option>
              <option value="F7">F7</option>
              <option value="F10">F10</option>
              <option value="F11">F11</option>
              <option value="F12">F12</option>
              <option value="ScrollLock">ScrollLock</option>
              <option value="Pause">Pause</option>
            </select>
          </div>

          <div>
            <label style={{ display: 'block', marginBottom: '4px', fontWeight: 'bold' }}>上下文記憶句數</label>
            <select
              value={contextSentencesInput}
              onChange={(e) => setContextSentencesInput(Number(e.target.value))}
              style={{ width: '100%', padding: '8px', border: '1px solid #ccc', borderRadius: '6px' }}
            >
              <option value={1}>1 句 (極速)</option>
              <option value={3}>3 句</option>
              <option value={5}>5 句 (標準建議)</option>
              <option value={8}>8 句</option>
              <option value={10}>10 句 (深層記憶)</option>
            </select>
          </div>
        </div>

        <div style={{ marginBottom: '16px', display: 'flex', alignItems: 'center', gap: '8px' }}>
          <input
            type="checkbox"
            id="autostart-chk"
            checked={autoStartInput}
            onChange={(e) => setAutoStartInput(e.target.checked)}
            style={{ width: '16px', height: '16px' }}
          />
          <label htmlFor="autostart-chk" style={{ fontWeight: 'bold', cursor: 'pointer' }}>開機自動啟動</label>
        </div>

        <div style={{ display: 'flex', gap: '8px' }}>
          <button
            onClick={() => setShowSettings(false)}
            style={{ flex: 1, padding: '10px', background: '#e5e7eb', color: '#374151', border: 'none', borderRadius: '6px', cursor: 'pointer', fontWeight: 'bold' }}
          >
            返回
          </button>
          <button
            onClick={saveSettings}
            style={{ flex: 2, padding: '10px', background: '#7c3aed', color: 'white', border: 'none', borderRadius: '6px', fontSize: '15px', cursor: 'pointer', fontWeight: 'bold' }}
          >
            儲存設定
          </button>
        </div>
      </div>
    )
  }

  // 畫面 2：歷程紀錄介面
  if (showHistory) {
    return (
      <div style={{ padding: '16px 20px', maxWidth: '380px', margin: '0 auto', fontSize: '13px' }}>
        <h2 style={{ color: '#7c3aed', textAlign: 'center', margin: '0 0 12px 0', fontSize: '18px' }}>連續上下文歷史紀錄</h2>
        <p style={{ color: '#6b7280', fontSize: '11px', marginBottom: '10px', textAlign: 'center' }}>
          AI 校正時會自動參考下列前文維持同音字與專有名詞連貫：
        </p>
        <div style={{ maxHeight: '250px', overflowY: 'auto', border: '1px solid #e5e7eb', borderRadius: '6px', padding: '10px', background: '#f9fafb' }}>
          {historyList.length === 0 ? (
            <p style={{ color: '#9ca3af', textAlign: 'center', padding: '20px 0' }}>尚無對話記憶紀錄</p>
          ) : (
            historyList.map((item, index) => (
              <div key={index} style={{ padding: '6px 0', borderBottom: index < historyList.length - 1 ? '1px dashed #e5e7eb' : 'none' }}>
                <span style={{ fontWeight: 'bold', color: '#7c3aed', marginRight: '6px' }}>{index + 1}.</span>
                <span>{item}</span>
              </div>
            ))
          )}
        </div>
        <div style={{ display: 'flex', gap: '8px', marginTop: '14px' }}>
          <button
            onClick={() => setShowHistory(false)}
            style={{ flex: 1, padding: '9px', background: '#e5e7eb', color: '#374151', border: 'none', borderRadius: '6px', cursor: 'pointer', fontWeight: 'bold' }}
          >
            返回
          </button>
          <button
            onClick={clearHistory}
            style={{ flex: 1, padding: '9px', background: '#ef4444', color: 'white', border: 'none', borderRadius: '6px', cursor: 'pointer', fontWeight: 'bold' }}
          >
            🗑️ 清空記憶
          </button>
        </div>
      </div>
    )
  }

  // 畫面 3：主打字視窗
  return (
    <div style={{ padding: '20px', maxWidth: '380px', margin: '0 auto', textAlign: 'center' }}>
      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '8px' }}>
        <span style={{ fontSize: '11px', fontWeight: 'bold', background: '#f3e8ff', color: '#7c3aed', padding: '2px 8px', borderRadius: '10px' }}>
          v1.0.4 旗艦版
        </span>
        <button
          onClick={() => { refreshHistory(); setShowHistory(true) }}
          style={{ background: 'none', border: 'none', color: '#6b7280', fontSize: '12px', cursor: 'pointer', textDecoration: 'underline' }}
        >
          📝 歷史紀錄 ({historyList.length})
        </button>
      </div>

      <h1 style={{ color: '#7c3aed', fontSize: '26px', margin: '4px 0 2px 0' }}>VoiceType</h1>
      <p style={{ color: '#666', fontSize: '13px', marginBottom: '24px' }}>全域 AI 繁中語音輸入工具</p>

      <button
        onClick={handleButton}
        style={{
          width: '96px',
          height: '96px',
          borderRadius: '50%',
          border: 'none',
          background: isRecording ? '#ef4444' : '#7c3aed',
          color: 'white',
          fontSize: '38px',
          cursor: 'pointer',
          margin: '0 auto',
          display: 'block',
          boxShadow: '0 4px 14px rgba(124, 58, 237, 0.35)',
          transition: 'transform 0.1s ease',
        }}
      >
        {isRecording ? '⏹' : '🎤'}
      </button>

      <p style={{ marginTop: '18px', color: isRecording ? '#ef4444' : '#1f2937', fontWeight: 'bold', fontSize: '14px', minHeight: '22px' }}>
        {status}
      </p>

      <div style={{ marginTop: '12px', fontSize: '12.5px', color: '#6b7280' }}>
        按鍵：<span style={{ fontWeight: 'bold', color: '#7c3aed', background: '#f3f4f6', padding: '2px 6px', borderRadius: '4px' }}>{config.hotkey || 'F9'}</span>（按住錄音，鬆開停止）
      </div>

      <div style={{ marginTop: '18px' }}>
        <button
          onClick={() => {
            setApiKeyInput(config.groq_api_key)
            setAutoStartInput(config.auto_start)
            setHotkeyInput(config.hotkey || 'F9')
            setContextSentencesInput(config.context_sentences || 5)
            setShowSettings(true)
          }}
          style={{ background: 'none', border: 'none', color: '#7c3aed', cursor: 'pointer', textDecoration: 'underline', fontSize: '13px' }}
        >
          ⚙️ 偏好設定
        </button>
      </div>
    </div>
  )
}

export default App
