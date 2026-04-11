import { useRef, useCallback, useEffect } from 'react'

function formatDuration(secs) {
  if (secs === 0) return 'Brak'
  if (secs < 60) return `${secs}s`
  const m = Math.floor(secs / 60)
  const s = secs % 60
  return s > 0 ? `${m}m ${s}s` : `${m} min`
}

// Repeat-on-hold hook — calls fn() immediately, then repeatedly after delay
function useRepeat(fn, repeatMs = 130, delayMs = 420) {
  const fnRef = useRef(fn)
  fnRef.current = fn  // always latest, no stale closure

  const timeoutRef = useRef(null)
  const repeatRef  = useRef(null)

  useEffect(() => () => {
    clearTimeout(timeoutRef.current)
    clearInterval(repeatRef.current)
  }, [])

  const start = useCallback(() => {
    fnRef.current()
    timeoutRef.current = setTimeout(() => {
      repeatRef.current = setInterval(() => fnRef.current(), repeatMs)
    }, delayMs)
  }, [repeatMs, delayMs])

  const stop = useCallback(() => {
    clearTimeout(timeoutRef.current)
    clearInterval(repeatRef.current)
  }, [])

  return [start, stop]
}

function NumControl({ value, onChange, min = 0, max = Infinity, step = 1, format }) {
  const inc = useCallback(() => onChange(Math.min(max, value + step)), [onChange, value, max, step])
  const dec = useCallback(() => onChange(Math.max(min, value - step)), [onChange, value, min, step])

  const [startInc, stopInc] = useRepeat(inc)
  const [startDec, stopDec] = useRepeat(dec)

  const bindBtn = (startFn, stopFn) => ({
    onMouseDown:  startFn,
    onMouseUp:    stopFn,
    onMouseLeave: stopFn,
    onTouchStart: (e) => { e.preventDefault(); startFn() },
    onTouchEnd:   stopFn,
    onTouchCancel:stopFn,
  })

  return (
    <div className="num-control">
      <button className="step-btn" {...bindBtn(startDec, stopDec)} disabled={value <= min}>
        −
      </button>
      <span className="num-value">
        {format ? format(value) : value}
      </span>
      <button className="step-btn" {...bindBtn(startInc, stopInc)} disabled={value >= max}>
        +
      </button>
    </div>
  )
}

function AlertSelect({ value, onChange }) {
  return (
    <select
      className="alert-select"
      value={value}
      onChange={e => onChange(e.target.value)}
    >
      <option value="none">Brak</option>
      <option value="vibrate">Wibracje</option>
      <option value="beep">Dzwiek</option>
      <option value="both">Oba</option>
    </select>
  )
}

function Toggle({ checked, onChange, label }) {
  return (
    <button
      className={`toggle ${checked ? 'on' : 'off'}`}
      onClick={() => onChange(!checked)}
      role="switch"
      aria-checked={checked}
      aria-label={label}
    >
      <span className="toggle-thumb" />
    </button>
  )
}

function Section({ title, children }) {
  return (
    <div className="settings-section">
      <div className="section-title">{title}</div>
      {children}
    </div>
  )
}

export default function SettingsView({ settings, onSettingsChange, onStart }) {
  const set = (key) => (val) =>
    onSettingsChange(prev => ({ ...prev, [key]: val }))

  return (
    <div className="settings-view">
      <h1 className="app-title">TIMER</h1>

      <Section title="CZAS RUNDY">
        <NumControl
          value={settings.roundDuration}
          onChange={set('roundDuration')}
          min={5} max={3600} step={5}
          format={formatDuration}
        />
      </Section>

      <Section title="ODPOCZYNEK">
        <NumControl
          value={settings.restDuration}
          onChange={set('restDuration')}
          min={0} max={3600} step={5}
          format={formatDuration}
        />
      </Section>

      <Section title="LICZBA RUND">
        <NumControl
          value={settings.numRounds}
          onChange={set('numRounds')}
          min={1} max={99} step={1}
        />
      </Section>

      <Section title="ODLICZANIE PRZED STARTEM">
        <NumControl
          value={settings.prepDuration}
          onChange={set('prepDuration')}
          min={0} max={60} step={1}
          format={v => v === 0 ? 'Wylaczone' : `${v}s`}
        />
      </Section>

      <Section title="ALERTY">
        <div className="alert-row">
          <span className="alert-label">Zmiana fazy</span>
          <AlertSelect
            value={settings.phaseChangeAlert}
            onChange={set('phaseChangeAlert')}
          />
        </div>

        <div className="alert-row">
          <span className="alert-label">Co jakis czas</span>
          <Toggle
            checked={settings.midAlertEnabled}
            onChange={set('midAlertEnabled')}
            label="Alert co jakis czas"
          />
        </div>

        {settings.midAlertEnabled && (
          <div className="alert-sub">
            <NumControl
              value={settings.midAlertInterval}
              onChange={set('midAlertInterval')}
              min={5} max={300} step={5}
              format={v => `co ${v}s`}
            />
            <AlertSelect
              value={settings.midAlertType}
              onChange={set('midAlertType')}
            />
          </div>
        )}

        <div className="alert-row">
          <span className="alert-label">Polowa rundy</span>
          <Toggle
            checked={settings.halfTimeAlert}
            onChange={set('halfTimeAlert')}
            label="Alert polowa rundy"
          />
        </div>

        {settings.halfTimeAlert && (
          <div className="alert-sub right">
            <AlertSelect
              value={settings.halfTimeAlertType}
              onChange={set('halfTimeAlertType')}
            />
          </div>
        )}
      </Section>

      <button className="start-btn" onClick={onStart}>
        START
      </button>
    </div>
  )
}
