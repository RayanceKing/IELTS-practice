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
  background:
    radial-gradient(circle at 7% -12%, rgba(129, 92, 246, 0.52), transparent 39rem),
    radial-gradient(circle at 96% 2%, rgba(59, 130, 246, 0.42), transparent 36rem),
    radial-gradient(circle at 51% 112%, rgba(124, 58, 237, 0.22), transparent 44rem),
    linear-gradient(145deg, #e8edff 0%, #edf2ff 46%, #dbeafe 100%);
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
  background:
    linear-gradient(115deg, rgba(255, 255, 255, 0.4), transparent 38%),
    repeating-linear-gradient(
      90deg,
      rgba(255, 255, 255, 0.16) 0,
      rgba(255, 255, 255, 0.16) 1px,
      transparent 1px,
      transparent 94px
    );
}

#shui-three-bg::after {
  width: min(62vw, 940px);
  height: min(62vw, 940px);
  right: -24vw;
  bottom: -34vw;
  border-radius: 999px;
  opacity: 0.34;
  filter: blur(3px);
  background: radial-gradient(circle, rgba(255, 255, 255, 0.92), rgba(190, 205, 255, 0.18) 46%, transparent 72%);
}

@media (prefers-contrast: more) {
  #shui-three-bg {
    background: #edf2ff;
  }

  #shui-three-bg::before,
  #shui-three-bg::after {
    display: none;
  }
}
</style>
