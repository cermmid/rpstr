import { useState, useCallback } from 'react'
import SettingsView from './SettingsView.jsx'
import TimerView from './TimerView.jsx'
import { useNotifications } from './hooks/useNotifications.js'
import { useTimer, PHASE } from './hooks/useTimer.js'

const DEFAULT_SETTINGS = {
  roundDuration:    180,  // seconds
  restDuration:      60,
  numRounds:          5,
  prepDuration:      10,
  phaseChangeAlert: 'vibrate', // 'none' | 'vibrate' | 'beep' | 'both'
  midAlertEnabled:  false,
  midAlertInterval:  60,
  midAlertType:     'vibrate',
  halfTimeAlert:    false,
  halfTimeAlertType:'vibrate',
}

export default function App() {
  const [settings, setSettings] = useState(DEFAULT_SETTINGS)
  const { notify, unlock } = useNotifications()
  const timer = useTimer(settings, notify)

  const handleStart = useCallback(() => {
    unlock()
    timer.start()
  }, [unlock, timer])

  if (timer.phase === PHASE.IDLE) {
    return (
      <SettingsView
        settings={settings}
        onSettingsChange={setSettings}
        onStart={handleStart}
      />
    )
  }

  return (
    <TimerView
      phase={timer.phase}
      timeLeft={timer.timeLeft}
      currentRound={timer.currentRound}
      isPaused={timer.isPaused}
      settings={settings}
      onPause={timer.pause}
      onResume={timer.resume}
      onStop={timer.stop}
    />
  )
}
