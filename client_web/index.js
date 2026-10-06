import init, { new_app } from './pkg/client_web.js'

const canvas = document.getElementById("canvas")

async function run() {
  await init()
  const app = await new_app(canvas)
  let resize = false
  const TICK_MS = 1000 / 60
  let last_frame = performance.now()
  let behind = 0

  const observer = new ResizeObserver(() => resize = true)
  observer.observe(canvas)

  function frame() {
    if (resize) {
      canvas.width = canvas.clientWidth
      canvas.height = canvas.clientHeight
      app.resize(canvas.width, canvas.height, 1)
      resize = false
    }
    const now = performance.now()
    behind = Math.min(behind + now - last_frame, TICK_MS * 5)
    last_frame = now
    while (behind >= TICK_MS) {
      app.tick()
      behind -= TICK_MS
    }
    app.draw()

    requestAnimationFrame(frame)
  }

  window.addEventListener('keydown', event => {
    app.key_down(event)
  })
  window.addEventListener('keyup', event => {
    app.key_up(event)
  })

  requestAnimationFrame(frame)
}

run()
