<template>
  <section
    class="relative m-3 hidden flex-col justify-between overflow-hidden bg-brown-ribbon-700 text-white p-8 lg:flex rounded-2xl w-5/6"
  >
    <div class="flex items-center gap-2 text-xl font-semibold">
      <WebcamIcon aria-hidden="true" />
      Orvix
    </div>

    <!-- decorative -->
    <div
      class="pointer-events-none absolute right-6 top-6 grid grid-cols-3 gap-1.5 opacity-40 items-center"
      aria-hidden="true"
    >
      <span
        v-for="i in 3"
        :key="i"
        class="h-3 w-0.5 bg-white/70"
        :class="i == 2 ? 'h-6' : ''"
      ></span>
    </div>
    <div
      class="pointer-events-none absolute bottom-6 left-6 grid grid-cols-4 gap-2 opacity-30"
      aria-hidden="true"
    >
      <span v-for="i in 6" :key="i" class="size-1 rounded-full bg-white"></span>
    </div>

    <div class="text-left h-1/2">
      <p class="text-3xl font-semibold tracking-tight" style="font-family: var(--font-heading)">
        Self-Hosted Video Meetings
      </p>
      <p class="mt-6 max-w-sm text-sm leading-relaxed">
        Your own private video conferencing platform. Full control, end-to-end encryption, and no
        third-party dependencies.
      </p>
      <div class="mt-5 flex justify-center gap-1.5" aria-hidden="true">
        <span class="size-1 rounded-full bg-white/40"></span>
        <span class="h-1 w-5 rounded-full bg-white/80"></span>
        <span class="size-1 rounded-full bg-white/40"></span>
      </div>
    </div>
  </section>
</template>
