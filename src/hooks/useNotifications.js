import { useRef, useCallback, useEffect } from 'react'

export function useNotifications() {
  const audioCtxRef = useRef(null)

  const getCtx = useCallback(() => {
    if (!audioCtxRef.current) {
      audioCtxRef.current = new (window.AudioContext || window.webkitAudioContext)()
    }
    const ctx = audioCtxRef.current
    if (ctx.state === 'suspended') ctx.resume()
    return ctx
  }, [])

  const playBeep = useCallback((type = 'mid') => {
    try {
      const ctx = getCtx()
      const now = ctx.currentTime

      const sequences = {
        phase: [
          { freq: 880, start: 0,    dur: 0.12 },
          { freq: 880, start: 0.20, dur: 0.12 },
        ],
        mid: [
          { freq: 660, start: 0, dur: 0.09 },
        ],
        done: [
          { freq: 523, start: 0,    dur: 0.18 },
          { freq: 659, start: 0.25, dur: 0.18 },
          { freq: 784, start: 0.50, dur: 0.30 },
        ],
      }

      const tones = sequences[type] ?? sequences.mid

      tones.forEach(({ freq, start, dur }) => {
        const osc = ctx.createOscillator()
        const gain = ctx.createGain()
        osc.connect(gain)
        gain.connect(ctx.destination)
        osc.type = 'sine'
        osc.frequency.value = freq
        const t = now + start
        gain.gain.setValueAtTime(0, t)
        gain.gain.linearRampToValueAtTime(0.35, t + 0.01)
        gain.gain.exponentialRampToValueAtTime(0.001, t + dur)
        osc.start(t)
        osc.stop(t + dur + 0.05)
      })
    } catch (err) {
      console.warn('Audio error:', err)
    }
  }, [getCtx])

  const vibrate = useCallback((type = 'mid') => {
    if (!navigator.vibrate) return
    const patterns = {
      phase: [120, 60, 120],
      mid:   [60],
      done:  [150, 80, 150, 80, 300],
    }
    navigator.vibrate(patterns[type] ?? patterns.mid)
  }, [])

  const notify = useCallback((type, alertConfig) => {
    if (!alertConfig || alertConfig === 'none') return
    if (alertConfig === 'vibrate' || alertConfig === 'both') vibrate(type)
    if (alertConfig === 'beep'    || alertConfig === 'both') playBeep(type)
  }, [vibrate, playBeep])

  // Unlock AudioContext on first user interaction
  const unlock = useCallback(() => {
    try { getCtx() } catch {}
  }, [getCtx])

  // Cleanup
  useEffect(() => () => {
    audioCtxRef.current?.close()
  }, [])

  return { notify, unlock }
}
