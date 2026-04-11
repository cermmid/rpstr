import { useState, useEffect, useRef, useCallback } from 'react'

export const PHASE = {
  IDLE: 'idle',
  PREP: 'prep',
  WORK: 'work',
  REST: 'rest',
  DONE: 'done',
}

export const PHASE_LABELS = {
  [PHASE.IDLE]: '',
  [PHASE.PREP]: 'Przygotowanie',
  [PHASE.WORK]: 'Runda',
  [PHASE.REST]: 'Odpoczynek',
  [PHASE.DONE]: 'Koniec!',
}

export const PHASE_COLORS = {
  [PHASE.IDLE]: '#888',
  [PHASE.PREP]: '#f0c040',
  [PHASE.WORK]: '#40d080',
  [PHASE.REST]: '#4a8fff',
  [PHASE.DONE]: '#c060f0',
}

export function useTimer(settings, notify) {
  const [phase, setPhase]               = useState(PHASE.IDLE)
  const [timeLeft, setTimeLeft]         = useState(0)
  const [currentRound, setCurrentRound] = useState(0)
  const [isPaused, setIsPaused]         = useState(false)

  // All mutable timer state in one ref to avoid stale closures
  const ts = useRef({
    phase: PHASE.IDLE,
    round: 0,
    endTime: 0,
    isPaused: false,
    halfFired: false,
    lastMidInterval: 0,
    remainingOnPause: 0,
  })

  const settingsRef = useRef(settings)
  useEffect(() => { settingsRef.current = settings }, [settings])

  const notifyRef = useRef(notify)
  useEffect(() => { notifyRef.current = notify }, [notify])

  const intervalRef = useRef(null)

  // Always-fresh tick via ref — no stale closure issues
  const tickRef = useRef(null)
  tickRef.current = () => {
    const state = ts.current
    const s = settingsRef.current

    if (state.isPaused || state.phase === PHASE.IDLE || state.phase === PHASE.DONE) return

    const remaining = Math.max(0, (state.endTime - Date.now()) / 1000)
    setTimeLeft(Math.ceil(remaining))

    // Mid-round alerts — only during WORK, not in the last 2 seconds
    if (state.phase === PHASE.WORK && remaining > 2) {
      const elapsed = s.roundDuration - remaining

      if (s.midAlertEnabled && s.midAlertInterval > 0) {
        const idx = Math.floor(elapsed / s.midAlertInterval)
        if (idx > 0 && idx > state.lastMidInterval) {
          state.lastMidInterval = idx
          notifyRef.current('mid', s.midAlertType)
        }
      }

      if (s.halfTimeAlert && !state.halfFired && remaining <= s.roundDuration / 2 + 0.3) {
        state.halfFired = true
        notifyRef.current('mid', s.halfTimeAlertType)
      }
    }

    if (remaining <= 0) {
      advancePhase()
    }
  }

  function advancePhase() {
    const state = ts.current
    const s = settingsRef.current

    if (state.phase === PHASE.PREP) {
      state.phase = PHASE.WORK
      state.round = 1
      state.endTime = Date.now() + s.roundDuration * 1000
      state.halfFired = false
      state.lastMidInterval = 0
      notifyRef.current('phase', s.phaseChangeAlert)
      setPhase(PHASE.WORK)
      setCurrentRound(1)
      setTimeLeft(s.roundDuration)

    } else if (state.phase === PHASE.WORK) {
      if (state.round >= s.numRounds) {
        state.phase = PHASE.DONE
        clearInterval(intervalRef.current)
        notifyRef.current('done', s.phaseChangeAlert)
        setPhase(PHASE.DONE)
        setTimeLeft(0)
      } else if (s.restDuration > 0) {
        state.phase = PHASE.REST
        state.endTime = Date.now() + s.restDuration * 1000
        notifyRef.current('phase', s.phaseChangeAlert)
        setPhase(PHASE.REST)
        setTimeLeft(s.restDuration)
      } else {
        // No rest — jump directly to next work round
        const next = state.round + 1
        state.phase = PHASE.WORK
        state.round = next
        state.endTime = Date.now() + s.roundDuration * 1000
        state.halfFired = false
        state.lastMidInterval = 0
        notifyRef.current('phase', s.phaseChangeAlert)
        setPhase(PHASE.WORK)
        setCurrentRound(next)
        setTimeLeft(s.roundDuration)
      }

    } else if (state.phase === PHASE.REST) {
      const next = state.round + 1
      state.phase = PHASE.WORK
      state.round = next
      state.endTime = Date.now() + s.roundDuration * 1000
      state.halfFired = false
      state.lastMidInterval = 0
      notifyRef.current('phase', s.phaseChangeAlert)
      setPhase(PHASE.WORK)
      setCurrentRound(next)
      setTimeLeft(s.roundDuration)
    }
  }

  const start = useCallback(() => {
    const s = settingsRef.current
    const state = ts.current

    const hasPrep = s.prepDuration > 0
    state.phase = hasPrep ? PHASE.PREP : PHASE.WORK
    state.round = hasPrep ? 0 : 1
    state.endTime = Date.now() + (hasPrep ? s.prepDuration : s.roundDuration) * 1000
    state.isPaused = false
    state.halfFired = false
    state.lastMidInterval = 0
    state.remainingOnPause = 0

    setPhase(state.phase)
    setCurrentRound(state.round)
    setTimeLeft(hasPrep ? s.prepDuration : s.roundDuration)
    setIsPaused(false)

    clearInterval(intervalRef.current)
    intervalRef.current = setInterval(() => tickRef.current?.(), 100)
  }, [])

  const pause = useCallback(() => {
    const state = ts.current
    state.isPaused = true
    state.remainingOnPause = Math.max(0, state.endTime - Date.now())
    setIsPaused(true)
  }, [])

  const resume = useCallback(() => {
    const state = ts.current
    state.isPaused = false
    state.endTime = Date.now() + state.remainingOnPause
    setIsPaused(false)
  }, [])

  const stop = useCallback(() => {
    clearInterval(intervalRef.current)
    ts.current.phase = PHASE.IDLE
    ts.current.isPaused = false
    setPhase(PHASE.IDLE)
    setTimeLeft(0)
    setCurrentRound(0)
    setIsPaused(false)
  }, [])

  useEffect(() => () => clearInterval(intervalRef.current), [])

  return { phase, timeLeft, currentRound, isPaused, start, pause, resume, stop }
}
