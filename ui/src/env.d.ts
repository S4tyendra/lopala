/// <reference types="astro/client" />

// Allow importing GLSL files as raw strings via Vite's ?raw suffix
declare module '*.glsl?raw' {
  const src: string
  export default src
}

// Also handle without suffix (direct import)
declare module '*.glsl' {
  const src: string
  export default src
}

declare module '*.vert' {
  const src: string
  export default src
}

declare module '*.frag' {
  const src: string
  export default src
}
