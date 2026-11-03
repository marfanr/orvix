<script setup lang="ts">
import { WebcamIcon } from 'lucide-vue-next'
</script>

<template>
  <div class="relative flex min-h-dvh w-full flex-col bg-gray-50">
    <header class="flex items-center gap-2 px-6 py-5 sm:px-10" aria-hidden="true">
      <WebcamIcon aria-hidden="true" />
      <span class="font-heading text-sm font-semibold text-gray-900">VidCall</span>
    </header>

    <main class="flex flex-1 flex-col items-center justify-center px-6 pb-16 text-center">
      <h1 class="font-heading text-2xl font-medium tracking-tight text-gray-900 sm:text-3xl">
        Page not found
      </h1>
      <p class="mt-2 max-w-sm text-sm text-gray-500">
        The page you're looking for doesn't exist or has been moved.
      </p>

      <div class="mt-10 flex items-center gap-x-3">
        <span class="h-px w-10 bg-gray-200" aria-hidden="true"></span>
        <p class="font-mono text-xs text-gray-600">404</p>
        <span class="h-px w-10 bg-gray-200" aria-hidden="true"></span>
      </div>
      <p class="mt-1.5 text-[11px] uppercase tracking-widest text-gray-400">Error</p>

      <RouterLink
        to="/"
        class="mt-10 flex min-h-10 cursor-pointer items-center gap-x-2 rounded-lg bg-primary px-4 py-2 text-sm font-medium text-white transition-colors duration-150 hover:bg-primary-hover focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-primary"
      >
        Back to home
      </RouterLink>
    </main>
  </div>
</template>
