<template>
  <!-- Kept as a structural layer so every route shares one deterministic canvas. -->
  <div id="shui-three-bg" class="atlas-liquid-background" aria-hidden="true"></div>
</template>

<style>
#shui-three-bg {
  position: fixed;
  inset: 0;
  z-index: 0;
  overflow: hidden;
  pointer-events: none;
  background: var(--atlas-canvas);
}

#shui-three-bg::before,
#shui-three-bg::after {
  position: absolute;
  content: '';
  pointer-events: none;
}

#shui-three-bg::before {
  inset: 0;
  opacity: 0.62;
  background: var(--atlas-canvas-overlay);
}

#shui-three-bg::after {
  width: min(62vw, 940px);
  height: min(62vw, 940px);
  right: -24vw;
  bottom: -34vw;
  border-radius: 999px;
  opacity: 0.34;
  filter: blur(3px);
  background: var(--atlas-canvas-orb);
}

@media (prefers-contrast: more) {
  #shui-three-bg {
    background: var(--atlas-canvas-solid);
  }

  #shui-three-bg::before,
  #shui-three-bg::after {
    display: none;
  }
}
</style>
