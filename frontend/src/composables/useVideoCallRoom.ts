import { computed, onMounted, onUnmounted, watch } from 'vue'
import { storeToRefs } from 'pinia'
import { api } from '@/lib/axios'
import { useAuthStore } from '@/stores/auth'
import { useCallStore } from '@/stores/call'
import { useNotifySound } from '@/lib/audio'
import { useLocalMedia } from './useLocalMedia'
import { useSignaling } from './useSignaling'
import { useWebRTC } from './useWebRTC'

export function useVideoCallRoom(getRoomParam: () => string | string[] | undefined) {
  const callStore = useCallStore()
  const {
    room,
    participants,
    chatMessages,
    floating,
    joinRequests,
    currentRequest,
    remainingRequests,
    memberChangesData,
    newMessageData,
  } = storeToRefs(callStore)

  const auth = useAuthStore()
  let participantSeq = 0

  const signaling = useSignaling()
  const notifySound = useNotifySound()

  // play a sound when the first join request arrives
  watch(
    () => joinRequests.value.length,
    (len, prev) => {
      if (prev === 0 && len > 0) notifySound.play()
    },
  )

  const onRemoteTrack = (e: RTCTrackEvent) => {
    console.log('new track', e)

    const stream = e.streams[0] ?? new MediaStream([e.track])
    const streamId = stream.id
    console.log('onRemoteTrack event:', e.track.kind, e.streams, streamId)
    const target =
      callStore.participants.find((p) => p.conn_id && streamId.includes(p.conn_id)) ??
      callStore.participants.find((p) => p.id !== 0)

    if (!target) {
      console.warn('Track diterima untuk user yang tidak dikenal, abaikan.', streamId)
      return
    }

    if (target.stream) {
      target.stream.addTrack(e.track)
      target.stream = new MediaStream(target.stream.getTracks())
    } else {
      target.stream = stream
    }
    if (e.track.kind === 'video') {
      target.cameraOff = false
    }
    if (e.track.kind === 'audio') {
      target.muted = false
    }
    e.track.onended = () => {
      if (e.track.kind === 'video') {
        target.cameraOff = true
      }
      if (e.track.kind === 'audio') {
        target.muted = true
      }
      console.log('ended track')
    }
  }

  const handleControlChannel = (c: MessageEvent): void => {
    const data = JSON.parse(c.data)
    switch (data.type) {
      case 'track_state': {
        const target = callStore.participants.find((p) => p.conn_id === data.from)
        if (target && data.kind === 'video') target.cameraOff = data.muted
        if (target && data.kind === 'audio') target.muted = data.muted
        console.log('track state updated : ', target?.cameraOff, target?.muted)
        break
      }
      case 'track_ended': {
        const target = callStore.participants.find((p) => p.conn_id === data.from)
        if (target && data.kind === 'video') {
          target.cameraOff = true
          target.stream?.getVideoTracks().forEach((t) => t.stop())
        }
        break
      }
    }
  }

  const handleIncomingChat = (msg: string, from: string, timestamp: number) => {
    callStore.pushChat(msg, from, false, timestamp)
    callStore.addNewMessage(msg, from)
  }

  const webrtc = useWebRTC(
    signaling.send,
    onRemoteTrack,
    handleControlChannel,
    handleIncomingChat,
  )
  const media = useLocalMedia(participants)

  const sendChat = (msg: string, target: string) => {
    callStore.pushChat(msg, 'me', true, Date.now())
    webrtc.sendChat(JSON.stringify({ msg: msg, target: target, time: Date.now() }))
  }

  const cameraOff = computed(() => media.getMe()?.cameraOff ?? true)
  const muted = computed(() => media.getMe()?.muted ?? true)

  const onJoinedRoom = async (): Promise<void> => {
    const pc = webrtc.peerCon.value
    if (!pc) return

    const stream = media.getMe()?.stream
    if (stream) webrtc.addLocalStream(stream)

    const sdp = await webrtc.createOffer()
    if (sdp) signaling.sendOffer(sdp)
  }

  const renegotiate = async (): Promise<void> => {
    const pc = webrtc.peerCon.value
    if (!pc || pc.signalingState !== 'stable') return

    const sdp = await webrtc.createOffer()
    if (sdp) signaling.sendOffer(sdp)
  }

  const toggleCamera = async () => {
    await media.toggleCamera()

    const stream = media.getMe()?.stream
    if (!stream) return

    await webrtc.addLocalStream(stream)
    await renegotiate()
  }

  const toggleMic = async () => {
    await media.toggleMic()

    const stream = media.getMe()?.stream
    if (!stream) return

    await webrtc.addLocalStream(stream)
    await renegotiate()
  }

  watch(
    () => room.value.status === 'joined',
    (joined, prevJoined) => {
      if (joined && !prevJoined) void onJoinedRoom()
    },
  )

  signaling.onMessage(async (msg) => {
    let type: string | undefined

    try {
      const data = JSON.parse(msg.data)
      type = data.type

      switch (data.type) {
        case 'reaction':
          if (typeof data.emoji === 'string') callStore.showReaction(data.emoji)
          break

        case 'join':
          if (data.accept) callStore.room.status = 'joined'
          else if (callStore.room.status !== 'joined') callStore.room.status = 'denied'
          webrtc.sendDataControl('test')
          break

        // synchronize current members
        case 'sync': {
          console.log(data)
          const members: { conn_id: string; username: string }[] = data.members

          for (const v of members) {
            if (callStore.participants.some((m) => m.conn_id === v.conn_id)) continue

            callStore.upsertParticipant({
              id: ++participantSeq,
              conn_id: v.conn_id,
              name: v.username,
              muted: true,
              cameraOff: true,
              stream: null,
            })
          }
          break
        }

        case 'request': {
          const { username, conn_id } = data.from
          callStore.addJoinRequest({ username, conn_id })
          break
        }

        case 'offer': {
          const answerSdp = await webrtc.handleRemoteOffer(data.sdp)
          if (answerSdp) {
            signaling.sendAnswer(answerSdp)
          }
          break
        }
        // handle offer from server publisher
        case 'answer': {
          console.log('answer', data)
          await webrtc.setRemoteAnswer(data.sdp)
          break
        }

        case 'ice': {
          webrtc.addIceCandidate(data.peer_type, {
            candidate: data.candidate,
            sdpMid: data.sdpMid,
            sdpMLineIndex: data.sdpMLineIndex,
            usernameFragment: data.usernameFragment,
          })
          break
        }

        case 'welcome': {
          const { username, conn_id } = data.from

          callStore.addMemberChange(username, 'join')
          callStore.upsertParticipant({
            id: ++participantSeq,
            conn_id,
            name: username,
            muted: true,
            cameraOff: true,
            stream: null,
          })
          break
        }

        /// notification from server indicates room host already joined the room
        case 'notify': {
          signaling.join(callStore.room.id!, auth.accessToken)
          break
        }

        // guest disconnect
        // TODO: not handled on backend
        case 'request_cancel':
          callStore.cancelJoinRequest(data.conn_id)
          break

        case 'disconnect': {
          callStore.removeParticipant(data.conn_id)
          callStore.addMemberChange(data.username, 'leave')
          break
        }
      }
    } catch (err) {
      console.error('signaling error:', type, err)
    }
  })

  const resolveRequest = (type: 'admit' | 'deny') => {
    const req = callStore.currentRequest
    if (!req) return
    // TODO!: handle deny
    if (type === 'admit') {
      signaling.answerRequest(callStore.room.id, req.conn_id)
    }
    callStore.shiftJoinRequest()
  }

  const hangup = () => {
    signaling.leave(callStore.room.id)
    webrtc.close()
  }

  onMounted(async () => {

    try {
      await webrtc.init()
    } catch (error) {
      console.error('webrtc init failed:', error)
      callStore.room.status = 'not-found'
      return
    }

    try {
      const res = await api.get(`/room/${getRoomParam()}`)
      callStore.room.id = res.data.id
      callStore.room.owner = res.data.owner
      callStore.room.status = res.data.is_started ? 'waiting' : 'not-started'
      callStore.room.title = res.data.title ?? 'Untitled room'

      await media.init()

      signaling.join(res.data.id, auth.accessToken)
    } catch (error) {
      callStore.room.status = 'not-found'
      console.error(error)
    }
  })

  onUnmounted(() => {
    webrtc.close()
  })

  return {
    // refs (state) — from the store
    participants,
    room,
    chatMessages,
    floating,
    currentRequest,
    remainingRequests,
    memberChangesData,
    newMessageData,
    // actions / orchestration
    showReaction: callStore.showReaction,
    resolveRequest,
    resolveNewMembers: callStore.resolveMemberChange,
    resolveNewMessage: callStore.resolveNewMessage,
    toggleCamera,
    toggleMic,
    cameraOff,
    muted,
    hangup,
    sendControl: webrtc.sendDataControl,
    sendChat,
  }
}
