// SPDX-FileCopyrightText: 2026 Mohammad Arfan
//
// SPDX-License-Identifier: AGPL-3.0

import { ref } from 'vue'
import { SupportedVideoCodec } from '@/types/video-call.ts'
import { useMeetingConfigStore } from '@/stores/meetingConfig'
import { isSvcSupported, preferCodec } from '@/lib/media'
import { storeToRefs } from 'pinia'

export type DataControlType = string | Blob | ArrayBuffer

type PcRole = 'pub' | 'sub'

export function useWebRTC(
  send: (obj: unknown) => unknown,
  onTrack?: (e: RTCTrackEvent) => void,
  handleControl?: (c: MessageEvent) => void,
  handleIncomingChat?: (msg: string, from: string, timestamp: number) => void,
  sendOffer?: (offer: string) => void,
) {
  const debug = (...args: unknown[]) => {
    if (import.meta.env.DEV) console.log(...args)
  }

  const store = useMeetingConfigStore()
  const { form } = storeToRefs(store)

  const remoteVideoRef = ref<HTMLVideoElement | null>(null)

  const PublisherPeerCon = ref<RTCPeerConnection | null>(null)
  const SubscriberPeerCon = ref<RTCPeerConnection | null>(null)

  const localTracksVideo = ref<RTCRtpSender | null>(null)
  const localTracksTranceiver = ref<RTCRtpTransceiver | null>(null)

  const localTracksAudio = ref<RTCRtpSender | null>(null)
  const controlChannelSub = ref<RTCDataChannel | null>(null)
  const controlChannelPub = ref<RTCDataChannel | null>(null)
  const chatChannelPub = ref<RTCDataChannel | null>(null)
  const chatChannelSub = ref<RTCDataChannel | null>(null)

  const pendingMessageControlCh: DataControlType[] = []

  const pendingCandidates: Record<PcRole, RTCIceCandidateInit[]> = { pub: [], sub: [] }
  const remoteDescSet: Record<PcRole, boolean> = { pub: false, sub: false }

  const pcOf = (role: PcRole) => (role === 'pub' ? PublisherPeerCon.value : SubscriberPeerCon.value)

  const flushPendingCandidates = async (role: PcRole) => {
    const queue = pendingCandidates[role]
    while (queue.length > 0) {
      const candidate = queue.shift()
      if (!candidate) continue
      try {
        await pcOf(role)?.addIceCandidate(candidate)
      } catch (err) {
        console.warn(`addIceCandidate failed (${role})`, err)
      }
    }
  }

  const init = async () => {
    if (PublisherPeerCon.value) return

    const iceServers: RTCIceServer[] = [{ urls: 'stun:stun.l.google.com:19302' }]

    const pub_pc = new RTCPeerConnection({
      iceServers,
      rtcpMuxPolicy: 'require',
    })
    const sub_pc = new RTCPeerConnection({
      iceServers,
      rtcpMuxPolicy: 'require',
    })

    PublisherPeerCon.value = pub_pc
    SubscriberPeerCon.value = sub_pc

    controlChannelSub.value ??= sub_pc.createDataChannel('control', { id: 0, negotiated: true })
    controlChannelPub.value ??= pub_pc.createDataChannel('control', { id: 0, negotiated: true })
    chatChannelPub.value ??= pub_pc.createDataChannel('chat', { id: 1, negotiated: true })
    chatChannelSub.value ??= sub_pc.createDataChannel('chat', { id: 1, negotiated: true })

    controlChannelSub.value.onmessage = (msg) => {
      debug('received control message')
      handleControl?.(msg)
    }

    chatChannelSub.value!.onmessage = (msg) => {
      const msg_data = JSON.parse(msg.data)
      debug('found new msg: ', msg.data, msg.timeStamp)
      handleIncomingChat?.(msg_data.msg, msg_data.from, msg_data.time)
    }

    const isIPv6 = (c: RTCIceCandidate) => {
      const addr = c.address ?? c.candidate.split(' ')[4]
      return addr?.includes(':')
    }

    pub_pc.addEventListener('icecandidate', (e) => {
      if (!e.candidate) return
      if (isIPv6(e.candidate)) return
      send({
        from: 'PUBLISHER',
        type: 'candidate',
        candidate: e.candidate.candidate,
        mid: e.candidate.sdpMid,
        line_index: e.candidate.sdpMLineIndex,
        username_fragment: e.candidate.usernameFragment,
      })
    })
    sub_pc.addEventListener('icecandidate', (e) => {
      if (!e.candidate) return
      if (isIPv6(e.candidate)) return
      send({
        from: 'SUBSCRIBER',
        type: 'candidate',
        candidate: e.candidate.candidate,
        mid: e.candidate.sdpMid,
        line_index: e.candidate.sdpMLineIndex,
        username_fragment: e.candidate.usernameFragment,
      })
    })

    pub_pc.addEventListener('iceconnectionstatechange', () => {
      const state = pub_pc.iceConnectionState
      console.log('ICE pub:', state)
      if (state === 'failed' || state === 'disconnected') {
        pub_pc.restartIce()
      }
    })
    sub_pc.addEventListener('iceconnectionstatechange', () => {
      const state = sub_pc.iceConnectionState
      console.log('ICE sub:', state)
      if (state === 'failed' || state === 'disconnected') {
        sub_pc.restartIce()
      }
    })

    const caps = RTCRtpReceiver.getCapabilities('video')
    // TODO: send capability to server
    debug('capability video ', caps)

    sub_pc.addEventListener('track', (e) => {
      if (remoteVideoRef.value)
        remoteVideoRef.value.srcObject = e.streams[0] ?? new MediaStream([e.track])
      onTrack?.(e)
    })
    pub_pc.addEventListener('datachannel', (e) => {
      const channel = e.channel
      console.log('new msg in datcan')

      channel.onmessage = (msg) => {
        handleControl?.(msg)
      }
    })

    let offering = false
    pub_pc.onnegotiationneeded = async () => {
      if (offering) return
      offering = true
      try {
        console.log('needed renegotiation')
        const sdp = await createOffer()
        if (sdp) sendOffer!(sdp)
      } catch (err) {
        console.error('[useWebRTC] createOffer/sendOffer failed:', err)
      } finally {
        offering = false
      }
    }
  }

  const getPrefredCodecsAndModes = async () => {
    const codecs = ['video/VP9', 'video/AV1', 'video/VP8', 'video/H264']
    const modes = ['L3T3_KEY', 'L2T3_KEY', 'L2T1', 'L1T3', 'L1T2', 'L1T1']

    for (const contentType of codecs) {
      for (const scalabilityMode of modes) {
        try {
          const info = await navigator.mediaCapabilities.encodingInfo({
            type: 'webrtc',
            video: {
              contentType,
              width: 640,
              height: 480,
              bitrate: 1_000_000,
              framerate: 30,
              scalabilityMode: scalabilityMode,
            },
          })

          if (info.supported) {
            console.log(contentType, scalabilityMode, info)
            // return [contentType, scalabilityMode]
            break
          }
        } catch {
          continue
        }
      }
    }
  }

  /// attach local media tracks
  const addLocalStream = async (stream: MediaStream): Promise<boolean> => {
    debug(
      '[useWebRTC] addLocalStream',
      stream.getTracks().map((t) => ({ kind: t.kind, id: t.id, readyState: t.readyState })),
    )
    const pc = PublisherPeerCon.value
    if (!pc) return false

    let needsNegotiation = false

    for (const track of stream.getTracks()) {
      if (track.readyState === 'ended') {
        console.warn('[useWebRTC] skip ended track', track.kind)
        continue
      }

      const selected = await getPrefredCodecsAndModes()

      const useSvc = (await isSvcSupported()) && form.value.svc
      console.log('selected', selected)

      if (track.kind === 'video') {
        const encodings = useSvc
          ? [
              {
                scalabilityMode: 'L3T3',
              },
            ]
          : [
              { rid: 'q', scaleResolutionDownBy: 3, maxBitrate: 500_000, maxFramerate: 30 },
              { rid: 'h', scaleResolutionDownBy: 1.5, maxBitrate: 1_500_000, maxFramerate: 30 },
              { rid: 'f', scaleResolutionDownBy: 1, maxBitrate: 2_500_000, maxFramerate: 30 },
            ]
        const tr = localTracksTranceiver.value
        if (!tr) {
          localTracksTranceiver.value = pc.addTransceiver(track, {
            direction: 'sendonly',
            streams: [stream],
            sendEncodings: encodings,
          })
          localTracksVideo.value = localTracksTranceiver.value.sender
          needsNegotiation = true
        } else if (tr.sender.track?.id !== track.id) {
          debug('[useWebRTC] replaceTrack video', tr.sender.track?.id, '->', track.id)
          await tr.sender.replaceTrack(track)
        }
      } else if (track.kind === 'audio') {
        if (!localTracksAudio.value) {
          localTracksAudio.value = pc.addTrack(track, stream)

          needsNegotiation = true
        } else if (localTracksAudio.value.track?.id !== track.id) {
          await localTracksAudio.value.replaceTrack(track)
        }
      }
    }

    console.log('need negotiation')
    return needsNegotiation
  }

  /// create offer + apply it locally, returns the sdp to be sent to the peer
  const createOffer = async (): Promise<string | undefined> => {
    const pc = PublisherPeerCon.value
    if (!pc) return undefined
    const codec =
      form.value.svc && (await isSvcSupported())
        ? SupportedVideoCodec.VP9
        : SupportedVideoCodec.H264

    await preferCodec(pc, codec, 'video')
    const offer = await pc.createOffer()
    await pc.setLocalDescription(offer)
    return pc.localDescription?.sdp ?? offer.sdp
  }

  const setRemoteAnswer = async (sdp: string) => {
    const pc = PublisherPeerCon.value
    if (!pc) return
    if (pc.signalingState !== 'have-local-offer') {
      console.warn('[useWebRTC] answer ignored, signalingState =', pc.signalingState)
      return
    }
    try {
      await pc.setRemoteDescription({ type: 'answer', sdp })
    } catch (err) {
      console.error('[useWebRTC] setRemoteDescription(answer) failed:', err)
      // recover: Chrome replaces a pending offer when a new offer is applied,
      // so a fresh offer unblocks the connection instead of staying stuck
      try {
        const sdp2 = await createOffer()
        if (sdp2) sendOffer?.(sdp2)
      } catch (retryErr) {
        console.error('[useWebRTC] recovery offer failed:', retryErr)
      }
      return
    }
    remoteDescSet.pub = true
    await flushPendingCandidates('pub')
  }

  const handleRemoteOffer = async (sdp: string): Promise<string | undefined> => {
    const pc = SubscriberPeerCon.value
    if (!pc) return undefined
    if (pc.signalingState !== 'stable') {
      console.warn('handleRemoteOffer while signalingState is', pc.signalingState)
      await pc.setLocalDescription({ type: 'rollback' })
    }

    await pc.setRemoteDescription({ type: 'offer', sdp })
    remoteDescSet.sub = true
    await flushPendingCandidates('sub')
    const answer = await pc.createAnswer()
    await pc.setLocalDescription(answer)
    return pc.localDescription?.sdp ?? answer.sdp
  }

  /// candidates may arrive before the remote description is applied, buffer them per PC
  const addIceCandidate = async (peer_type: string, candidate: RTCIceCandidateInit) => {
    // candidate dari PUBLISHER server -> untuk subscriber PC kita, dan sebaliknya
    const role: PcRole = peer_type == 'PUBLISHER' ? 'sub' : 'pub'

    if (!remoteDescSet[role]) {
      pendingCandidates[role].push(candidate)
      return
    }
    try {
      await pcOf(role)?.addIceCandidate(candidate)
    } catch (err) {
      console.warn(`addIceCandidate failed (${role})`, err)
    }
  }

  const sendDataControl = <T extends DataControlType>(msg: T): void => {
    const channel = controlChannelPub.value!
    if (typeof msg === 'string') channel.send(msg)
    else if (msg instanceof Blob) channel.send(msg)
    else channel.send(msg)
  }

  const sendChat = <T extends DataControlType>(msg: T): void => {
    const channel = chatChannelPub.value!
    if (typeof msg === 'string') channel.send(msg)
    else if (msg instanceof Blob) channel.send(msg)
    else channel.send(msg)
  }

  const close = () => {
    controlChannelSub.value?.close()
    chatChannelPub.value?.close()
    chatChannelSub.value?.close()
    PublisherPeerCon.value?.close()
    SubscriberPeerCon.value?.close()

    PublisherPeerCon.value = null
    SubscriberPeerCon.value = null
    controlChannelSub.value = null
    chatChannelPub.value = null
    chatChannelSub.value = null

    localTracksVideo.value = null
    localTracksAudio.value = null
    localTracksTranceiver.value = null

    pendingCandidates.pub.length = 0
    pendingCandidates.sub.length = 0
    pendingMessageControlCh.length = 0

    remoteDescSet.pub = false
    remoteDescSet.sub = false
  }

  const addVideoTransceiver = (
    pc: RTCPeerConnection,
    track: MediaStreamTrack,
    stream: MediaStream,
  ): RTCRtpTransceiver => {
    try {
      return pc.addTransceiver(track, {
        direction: 'sendonly',
        streams: [stream],
        // sama dengan transceiver awal: rid q/h/f, supaya pemetaan layer
        // simulcast di server (FORWARD_RID = "q") tetap konsisten
        sendEncodings: [
          { rid: 'q', scaleResolutionDownBy: 3, maxBitrate: 500_000, maxFramerate: 30 },
          { rid: 'h', scaleResolutionDownBy: 1.5, maxBitrate: 1_500_000, maxFramerate: 30 },
          { rid: 'f', scaleResolutionDownBy: 1, maxBitrate: 2_500_000, maxFramerate: 30 },
        ],
      })
    } catch (err) {
      console.warn('addTransceiver with preferred encodings failed, fallback to default', err)
      return pc.addTransceiver(track, { direction: 'sendonly', streams: [stream] })
    }
  }

  const newTransceiver = (stream: MediaStream) => {
    const pc = PublisherPeerCon.value
    const track = stream.getVideoTracks().find((t) => t.readyState === 'live')
    if (!pc || !track) return false

    const old = localTracksTranceiver.value
    const created = addVideoTransceiver(pc, track, stream)

    localTracksTranceiver.value = created
    localTracksVideo.value = created.sender

    old?.stop()
    console.log('old', old?.currentDirection)
  }

  /// switch video codec WITHOUT replacing the transceiver: stopping the old one
  /// leaves a port-0 m-line in the next offer that the server mishandles, while
  /// the same transceiver keeps its mid/ssrc and only renegotiates the codec set
  const switchVideoCodec = (codec: SupportedVideoCodec): void => {
    const pc = PublisherPeerCon.value
    if (!pc) return
    void preferCodec(pc, codec).then(() => {
      sendDataControl(
        JSON.stringify({
          type: 'codec_changed',
          codec: codec.toString(),
          mime: `video/${codec.toString()}`,
        }),
      )
    })
  }

  return {
    remoteVideoRef,
    peerCon: PublisherPeerCon,
    init,
    addLocalStream,
    createOffer,
    setRemoteAnswer,
    handleRemoteOffer,
    addIceCandidate,
    close,
    localTracksVideo,
    localTracksAudio,
    sendDataControl,
    sendChat,
    newTransceiver,
    switchVideoCodec,
  }
}
