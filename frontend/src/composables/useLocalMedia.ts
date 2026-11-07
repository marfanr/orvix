// SPDX-FileCopyrightText: 2026 Mohammad Arfan
//
// SPDX-License-Identifier: AGPL-3.0

import { ref, type Ref } from 'vue'
import type { Participant } from '@/types/video-call'


const VIDEO_CONSTRAINTS: MediaTrackConstraints = {
  width: { ideal: 1280 },
  height: { ideal: 720 },
  frameRate: { ideal: 30, max: 30 },
}

export function useLocalMedia(participants: Ref<Participant[]>) {
  let localStream: MediaStream | null = null

  const cameraOn = ref(false)
  const micOn = ref(false)

  const busy = { camera: false, mic: false }

  const getMe = () => participants.value.find((p) => p.id == 0)

  const publish = () => {
    const me = getMe()
    if (!me) return
    const tracks = localStream?.getTracks() ?? []
    me.stream = tracks.length > 0 ? new MediaStream(tracks) : null
  }

  const init = async () => {
    if (!cameraOn.value && !micOn.value) return
    try {
      localStream = await navigator.mediaDevices.getUserMedia({
        video: cameraOn.value ? VIDEO_CONSTRAINTS : false,
        audio: micOn.value,
      })
      publish()
    } catch (error) {
      console.error(error)
    }
  }

  const toggleCamera = async () => {
    const me = getMe()
    if (!me || busy.camera) return
    busy.camera = true

    try {
      // turn off
      if (!me.cameraOff) {
        localStream?.getVideoTracks().forEach((t) => {
          t.stop()
          localStream?.removeTrack(t)
        })
        me.cameraOff = true
        publish()
        return
      }

      // turn on
      const stream = await navigator.mediaDevices.getUserMedia({
        video: VIDEO_CONSTRAINTS,
        audio: false,
      })

      const videoTrack = stream.getVideoTracks()[0]
      if (!videoTrack) return

      localStream ??= new MediaStream()
      localStream.addTrack(videoTrack)
      me.cameraOff = false
      publish()
    } catch (e) {
      console.error(e)
    } finally {
      busy.camera = false
    }
  }

  const toggleMic = async () => {
    const me = getMe()
    if (!me || busy.mic) return
    busy.mic = true

    try {
      // mute
      if (!me.muted) {
        localStream?.getAudioTracks().forEach((t) => {
          t.stop()
          localStream?.removeTrack(t)
        })
        me.muted = true
        publish()
        return
      }

      // unmute
      const stream = await navigator.mediaDevices.getUserMedia({
        video: false,
        audio: true,
      })

      const audioTrack = stream.getAudioTracks()[0]
      if (!audioTrack) return

      localStream ??= new MediaStream()
      localStream.addTrack(audioTrack)
      me.muted = false
      publish()
    } catch (e) {
      console.error(e)
    } finally {
      busy.mic = false
    }
  }

  return { cameraOn, micOn, init, toggleCamera, toggleMic, getMe }
}
