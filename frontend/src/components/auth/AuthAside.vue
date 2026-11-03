<script setup lang="ts">
defineProps<{
  headline: string
  description: string
}>()
</script>

<template>
  <aside
    class="relative hidden flex-col justify-between overflow-hidden bg-primary p-10 text-white lg:flex"
  >
    <!-- blueprint grid -->
    <div
      class="pointer-events-none absolute inset-0 opacity-[0.07]"
      style="
        background-image:
          linear-gradient(to right, white 1px, transparent 1px),
          linear-gradient(to bottom, white 1px, transparent 1px);
        background-size: 44px 44px;
      "
      aria-hidden="true"
    ></div>

    <!-- brand -->
    <div class="relative z-10 flex items-center gap-2.5">
      <span class="text-lg font-semibold tracking-tight">VidCall</span>
    </div>

    <!-- copy -->
    <div class="relative z-10">
      <p class="max-w-sm text-3xl leading-snug font-semibold tracking-tight">
        {{ headline }}
      </p>
      <p class="mt-3 max-w-sm text-sm leading-relaxed text-white/70">
        {{ description }}
      </p>
    </div>
  </aside>
</template>
