<script setup lang="ts">
import {
  CircleDotIcon,
  ScreenShareIcon,
  ShieldCheckIcon,
  UsersIcon,
  WebcamIcon,
} from 'lucide-vue-next'

const features = [
  { icon: UsersIcon, label: 'Multi-party WebRTC rooms' },
  { icon: ScreenShareIcon, label: 'Screen sharing, chat, and reactions' },
  { icon: ShieldCheckIcon, label: 'Waiting room and join requests' },
  { icon: CircleDotIcon, label: 'Server-side recording' },
]
</script>

<template>
  <section
    class="m-3 hidden flex-col justify-between overflow-hidden rounded-2xl bg-brown-ribbon-700 p-8 text-white lg:flex"
  >
    <div class="flex items-center gap-2 text-xl font-semibold">
      <WebcamIcon class="size-5" aria-hidden="true" />
      Orvix
    </div>

    <div>
      <p class="font-heading text-3xl font-semibold tracking-tight">Self-hosted video meetings</p>
      <p class="mt-4 max-w-sm text-sm leading-relaxed text-white/80">
        Run meetings on your own server. Multi-party WebRTC, screen sharing, and chat without a
        third-party meeting service.
      </p>

      <ul class="mt-8 space-y-3">
        <li
          v-for="feature in features"
          :key="feature.label"
          class="flex items-center gap-x-3 text-sm text-white/85"
        >
          <span class="grid size-8 shrink-0 place-items-center rounded-lg bg-white/10">
            <component :is="feature.icon" class="size-4" aria-hidden="true" />
          </span>
          {{ feature.label }}
        </li>
      </ul>
    </div>
  </section>
</template>
