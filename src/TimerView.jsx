import { PHASE, PHASE_LABELS, PHASE_COLORS } from './hooks/useTimer.js'

function formatTime(secs) {
  const s = Math.max(0, secs)
  const m = Math.floor(s / 60)
  const sec = s % 60
  return `${String(m).padStart(2, '0')}:${String(sec).padStart(2, '0')}`
}

function ProgressRing({ progress, color }) {
  const cx = 140, cy = 140, r = 126, stroke = 14
  const circ = 2 * Math.PI * r
  const offset = circ * (1 - Math.max(0, Math.min(1, progress)))

  return (
    <svg
      viewBox="0 0 280 280"
      style={{ width: '100%', height: '100%', transform: 'rotate(-90deg)' }}
      aria-hidden="true"
    >
      <circle
        cx={cx} cy={cy} r={r}
        fill="none" stroke="#252525" strokeWidth={stroke}
      />
      <circle
        cx={cx} cy={cy} r={r}
        fill="none" stroke={color} strokeWidth={stroke}
        strokeDasharray={circ}
        strokeDashoffset={offset}
        strokeLinecap="round"
        style={{ transition: 'stroke-dashoffset 0.15s linear, stroke 0.4s ease' }}
      />
    </svg>
  )
}

export default function TimerView({
  phase, timeLeft, currentRound, isPaused,
  settings, onPause, onResume, onStop,
}) {
  const color = PHASE_COLORS[phase] ?? '#888'
  const label = PHASE_LABELS[phase] ?? ''
  const isDone = phase === PHASE.DONE

  const phaseDuration =
    phase === PHASE.PREP ? settings.prepDuration :
    phase === PHASE.WORK ? settings.roundDuration :
    phase === PHASE.REST ? settings.restDuration : 1

  const progress = isDone ? 1 : 1 - timeLeft / phaseDuration

  return (
    <div className="timer-view">
      <div className="timer-topbar">
        <button className="stop-btn" onClick={onStop}>
          Zatrzymaj
        </button>
        {isPaused && <span className="paused-badge">PAUZA</span>}
      </div>

      <div className="ring-wrapper">
        <ProgressRing progress={progress} color={color} />

        <div className="ring-content">
          <div className="phase-label" style={{ color }}>
            {label}
          </div>

          {(phase === PHASE.WORK || phase === PHASE.REST) && (
            <div className="round-counter">
              {currentRound} / {settings.numRounds}
            </div>
          )}

          <div className="time-display" aria-live="polite">
            {isDone ? '---' : formatTime(timeLeft)}
          </div>

          {isDone && (
            <div className="done-subtext">
              Wszystkie rundy ukonczone
            </div>
          )}
        </div>
      </div>

      <div className="timer-controls">
        {isDone ? (
          <button className="ctrl-btn ctrl-primary" onClick={onStop}>
            Powrot do ustawien
          </button>
        ) : isPaused ? (
          <button className="ctrl-btn ctrl-primary" onClick={onResume}>
            Wznow
          </button>
        ) : (
          <button className="ctrl-btn ctrl-secondary" onClick={onPause}>
            Pauza
          </button>
        )}
      </div>
    </div>
  )
}
