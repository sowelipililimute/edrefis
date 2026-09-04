import init, { new_app } from './pkg/edrefis_web.js'

const canvas = document.getElementById("canvas")

async function run() {
  await init()
  const app = await new_app(canvas)
  let resize = false
  let last_tick = performance.now()

  const observer = new ResizeObserver(() => resize = true)
  observer.observe(canvas)

  function frame() {
    if (resize) {
      canvas.width = canvas.clientWidth
      canvas.height = canvas.clientHeight
      app.resize(canvas.width, canvas.height)
      resize = false
    }
    if ((last_tick - performance.now()) <= 1) {
      app.tick()
      last_tick = performance.now()
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
