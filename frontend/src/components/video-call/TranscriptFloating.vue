<script setup lang="ts">
import { useCallStore } from '@/stores/call'
import { storeToRefs } from 'pinia'
import { computed } from 'vue'

const store = useCallStore()
const { transcript } = storeToRefs(store)
const showTranscript = computed(() => transcript.value.length > 0)
</script>

<template>
  <Teleport to="body">
    <div class="absolute bottom-25 left-0 right-0 flex justify-center" v-if="showTranscript">
      <div class="flex flex-wrap gap-3 bg-black/80  rounded-md">
        <div class=" text-white p-1" v-for="t in transcript" :key="t.id">
          {{ t.text }}
        </div>
      </div>
    </div>
  </Teleport>
</template>
