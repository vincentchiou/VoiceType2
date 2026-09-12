import { useState, useEffect, useCallback, useRef } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'

interface AppConfig {
  groq_api_key: string
  auto_correct: boolean
  auto_start: boolean
}

function App() {
  const [config, setConfig] = useState<AppConfig>({ groq_api_key: '', auto_correct: true, auto_start: false })
  const [isRecording, setIsRecording] = useState(false)
  const [status, setStatus] = useState('載入中...')
  const [showSettings, setShowSettings] = useState(false)
  const [apiKeyInput, setApiKeyInput] = useState('')
  const [autoStartInput, setAutoStartInput] = useState(false)
  const isRecordingRef = useRef(false)
  const processingRef = useRef(false)

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
    } catch (err) {
      setStatus(`錯誤: ${err}`)
    } finally {
      processingRef.current = false
    }
  }, [])

  useEffect(() => {
    invoke<AppConfig>('get_config_command').then((cfg) => {
      setConfig(cfg)
      setApiKeyInput(cfg.groq_api_key)
      setAutoStartInput(cfg.auto_start)
      // 以登錄檔實際狀態為準
      invoke<boolean>('get_autostart').then(setAutoStartInput).catch(() => {})
      if (!cfg.groq_api_key) {
        setShowSettings(true)
        setStatus('請先設定 API Key')
      } else {
        setStatus('就緒 - 按 F9 開始錄音')
      }
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
  }, [startRec, stopRec])

  const handleButton = async () => {
    if (isRecordingRef.current) await stopRec()
    else await startRec()
  }

  const saveSettings = async () => {
    try {
      const newConfig = { groq_api_key: apiKeyInput, auto_correct: config.auto_correct, auto_start: autoStartInput }
      await invoke('set_config', { config: newConfig })
      setConfig(newConfig)
      setShowSettings(false)
      setStatus('就緒 - 按 F9 開始錄音')
    } catch (err) {
      setStatus(`儲存失敗: ${err}`)
    }
  }

  if (showSettings) {
    return (
      <div style={{ padding: '20px', maxWidth: '400px', margin: '0 auto' }}>
        <h2 style={{ color: '#7c3aed', textAlign: 'center' }}>VoiceType 設定</h2>
        <div style={{ marginTop: '20px' }}>
          <label style={{ display: 'block', marginBottom: '5px', fontWeight: 'bold' }}>Groq API Key</label>
          <input type="password" value={apiKeyInput} onChange={(e) => setApiKeyInput(e.target.value)}
            style={{ width: '100%', padding: '10px', border: '1px solid #ccc', borderRadius: '8px', boxSizing: 'border-box' }}
            placeholder="gsk_..." />
          <p style={{ fontSize: '12px', color: '#666', marginTop: '5px' }}>免費申請: https://console.groq.com</p>
        </div>
        <div style={{ marginTop: '15px', display: 'flex', alignItems: 'center', gap: '8px' }}>
          <input type="checkbox" checked={autoStartInput} onChange={(e) => setAutoStartInput(e.target.checked)}
            style={{ width: '18px', height: '18px' }} />
          <label style={{ fontWeight: 'bold' }}>開機自動啟動</label>
        </div>
        <p style={{ fontSize: '12px', color: '#666', marginTop: '5px' }}>Windows 重開機後自動執行 VoiceType</p>
        <button onClick={saveSettings}
          style={{ width: '100%', marginTop: '20px', padding: '12px', background: '#7c3aed', color: 'white', border: 'none', borderRadius: '8px', fontSize: '16px', cursor: 'pointer' }}>
          儲存
        </button>
      </div>
    )
  }

  return (
    <div style={{ padding: '20px', maxWidth: '400px', margin: '0 auto', textAlign: 'center' }}>
      <h1 style={{ color: '#7c3aed', fontSize: '28px' }}>VoiceType</h1>
      <p style={{ color: '#666', marginBottom: '30px' }}>AI 語音輸入工具</p>
      <button onClick={handleButton}
        style={{
          width: '100px', height: '100px', borderRadius: '50%', border: 'none',
          background: isRecording ? '#ef4444' : '#7c3aed', color: 'white',
          fontSize: '40px', cursor: 'pointer', margin: '0 auto', display: 'block',
          boxShadow: '0 4px 12px rgba(0,0,0,0.2)',
        }}>
        {isRecording ? '⏹' : '🎤'}
      </button>
      <p style={{ marginTop: '20px', color: isRecording ? '#ef4444' : '#333', fontWeight: 'bold' }}>{status}</p>
      <div style={{ marginTop: '15px', fontSize: '12px', color: '#999' }}>快捷鍵: F9 (按住錄音，鬆開停止)</div>
      <button onClick={() => { setApiKeyInput(config.groq_api_key); setAutoStartInput(config.auto_start); setShowSettings(true) }}
        style={{ marginTop: '15px', background: 'none', border: 'none', color: '#7c3aed', cursor: 'pointer', textDecoration: 'underline' }}>
        設定
      </button>
    </div>
  )
}

export default App
