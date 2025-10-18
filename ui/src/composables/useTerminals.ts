import { nextTick } from 'vue'
import { Terminal } from 'xterm'
import { FitAddon } from 'xterm-addon-fit'
import 'xterm/css/xterm.css'
import { windows, wsSend, myId } from './useWs'

const terminals = new Map<string, { term: Terminal; fitAddon: FitAddon; ro: ResizeObserver }>()

export function getTerminal(id: string) {
  return terminals.get(id)
}

export function initTerminal(id: string) {
  if (terminals.has(id)) return

  const el = document.getElementById(`term-${id}`)
  if (!el) return

  const term = new Terminal({
    cursorBlink: true,
    fontSize: 14,
    fontFamily: '"JetBrains Mono Nerd Font", "JetBrains Mono", "Cascadia Code", monospace',
    theme: {
      background: '#0c0c0c',
      foreground: '#d4d4d4',
      cursor: '#00ffcc',
      selectionBackground: '#ffffff33',
    },
    allowTransparency: true,
  })

  const fitAddon = new FitAddon()
  term.loadAddon(fitAddon)
  term.open(el)

  // Use ResizeObserver so terminal always fits its container — handles window resize & maximize
  const ro = new ResizeObserver(() => {
    try {
      fitAddon.fit()
      wsSend({ type: 'PtyResize', id, rows: term.rows, cols: term.cols })
    } catch {}
  })
  ro.observe(el)

  fitAddon.fit()
  terminals.set(id, { term, fitAddon, ro })

  // Request history for this session only (server sends directly back, not broadcast)
  wsSend({ type: 'RequestHistory', id })

  term.onData((data) => {
    wsSend({ type: 'PtyIn', id, data })
  })
}

export function writeToTerminal(id: string, data: string) {
  terminals.get(id)?.term.write(data)
}

export function disposeTerminal(id: string) {
  const t = terminals.get(id)
  if (!t) return
  t.ro.disconnect()
  t.term.dispose()
  terminals.delete(id)
}

export function checkAndInitTerminals(workspaceId: number) {
  // Dispose removed
  for (const id of terminals.keys()) {
    if (!windows.value[id]) disposeTerminal(id)
  }
  // Init new ones in current workspace
  for (const [id, win] of Object.entries(windows.value)) {
    if (win.app === 'terminal' && win.workspace === workspaceId && !terminals.has(id)) {
      nextTick(() => initTerminal(id))
    }
  }
}
