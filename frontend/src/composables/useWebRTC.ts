import { ref } from 'vue'
import { SupportedVideoCodec } from '@/types/video-call.ts'
import { useMeetingConfigStore } from '@/stores/meetingConfig'

export type DataControlType = string | Blob | ArrayBuffer

type PcRole = 'pub' | 'sub'

type BitrateSetting = { mode: 'keep' } | { mode: 'unlimited' } | { mode: 'cap'; bps: number }

const LAYER_SHARE: Record<string, number> = { q: 0.2, h: 0.6, f: 1 }

const resolveBitrate = (v: unknown): BitrateSetting => {
  switch (v) {
    case 'auto':
      return { mode: 'keep' }
    case '1':
      return { mode: 'cap', bps: 1_000_000 }
    case '2':
      return { mode: 'cap', bps: 2_000_000 }
    case '4':
      return { mode: 'cap', bps: 4_000_000 }
    case '8':
      return { mode: 'cap', bps: 8_000_000 }
    default:
      return { mode: 'unlimited' }
  }
}

export function useWebRTC(
  send: (obj: unknown) => unknown,
  onTrack?: (e: RTCTrackEvent) => void,
  handleControl?: (c: MessageEvent) => void,
  handleIncomingChat?: (msg: string, from: string, timestamp: number) => void,
) {
  const debug = (...args: unknown[]) => {
    if (import.meta.env.DEV) console.log(...args)
  }

  const form = useMeetingConfigStore().form

  const remoteVideoRef = ref<HTMLVideoElement | null>(null)

  const PublisherPeerCon = ref<RTCPeerConnection | null>(null)
  const SubscriberPeerCon = ref<RTCPeerConnection | null>(null)

  const localTracksVideo = ref<RTCRtpSender | null>(null)
  const localTracksTranceiver = ref<RTCRtpTransceiver | null>(null)

  const localTracksAudio = ref<RTCRtpSender | null>(null)
  const controlChannel = ref<RTCDataChannel | null>(null)
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

    controlChannel.value ??= sub_pc.createDataChannel('control', { id: 0, negotiated: true })
    chatChannelPub.value ??= pub_pc.createDataChannel('chat', { id: 1, negotiated: true })
    chatChannelSub.value ??= sub_pc.createDataChannel('chat', { id: 1, negotiated: true })

    controlChannel.value.onopen = () => {
      debug('WebRTC connected')
      while (pendingMessageControlCh.length > 0 && controlChannel.value?.readyState === 'open') {
        const msg = pendingMessageControlCh[0]

        try {
          _sendDataControl(msg!)
          pendingMessageControlCh.shift()
        } catch {
          break
        }
      }
    }
    controlChannel.value.onmessage = (msg) => {
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
      console.log('ICE pub:', pub_pc.iceConnectionState)
    })
    sub_pc.addEventListener('iceconnectionstatechange', () => {
      console.log('ICE sub:', sub_pc.iceConnectionState)
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

      channel.onmessage = (msg) => {
        handleControl?.(msg)
      }
    })
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

      if (track.kind === 'video') {
        const tr = localTracksTranceiver.value
        if (!tr) {
          localTracksTranceiver.value = pc.addTransceiver(track, {
            direction: 'sendonly',
            streams: [stream],
            sendEncodings: [
              { rid: 'q', scaleResolutionDownBy: 3, maxBitrate: 500_000, maxFramerate: 30 },
              { rid: 'h', scaleResolutionDownBy: 1.5, maxBitrate: 1_500_000, maxFramerate: 30 },
              { rid: 'f', scaleResolutionDownBy: 1, maxBitrate: 2_500_000, maxFramerate: 30 },
            ],
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

    return needsNegotiation
  }

  const preferCodec = async (
    pc: RTCPeerConnection,
    codec: SupportedVideoCodec,
    kind: string = 'video',
  ): Promise<void> => {
    codec ??= SupportedVideoCodec.VP9
    debug(`sellected = ${kind}/${codec.toString()}`)
    if (typeof RTCRtpSender.getCapabilities !== 'function') return

    const codecs = RTCRtpSender.getCapabilities(kind)?.codecs ?? []

    const sellectedCodec = codecs.filter(
      (c) => c.mimeType.toLowerCase() === `${kind}/${codec.toString().toLowerCase()}`,
    )
    if (sellectedCodec.length === 0) {
      console.warn(`${kind}/${codec.toString()} is not available, recording will not work`)
      return
    }

    const bitrate = resolveBitrate(form.maxBitrate)

    if (bitrate.mode !== 'keep') {
      for (const s of pc.getSenders()) {
        if (s.track?.kind !== 'video') continue

        const p = s.getParameters()
        if (!p.encodings || p.encodings.length === 0) continue

        for (const enc of p.encodings) {
          if (bitrate.mode === 'unlimited') {
            delete enc.maxBitrate
          } else {
            const share = (enc.rid && LAYER_SHARE[enc.rid]) || 1
            enc.maxBitrate = Math.round(bitrate.bps * share)
          }
        }

        try {
          await s.setParameters(p)
        } catch (err) {
          console.warn('setParameters failed', err)
        }
      }
    }

    for (const transceiver of pc.getTransceivers()) {
      const trKind = transceiver.sender.track?.kind ?? transceiver.receiver.track?.kind
      if (trKind !== 'video') continue

      try {
        transceiver.setCodecPreferences(sellectedCodec)
      } catch (err) {
        console.warn('setCodecPreferences failed', err)
      }
    }
  }

  /// create offer + apply it locally, returns the sdp to be sent to the peer
  const createOffer = async (): Promise<string | undefined> => {
    const pc = PublisherPeerCon.value
    if (!pc) return undefined
    await preferCodec(pc, form.preferredCodec, 'video')
    const offer = await pc.createOffer()
    await pc.setLocalDescription(offer)
    return pc.localDescription?.sdp ?? offer.sdp
  }

  const setRemoteAnswer = async (sdp: string) => {
    const pc = PublisherPeerCon.value
    if (!pc) return
    await pc.setRemoteDescription({ type: 'answer', sdp })
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

  const _sendDataControl = <T extends DataControlType>(msg: T): void => {
    const channel = controlChannel.value!
    if (typeof msg === 'string') channel.send(msg)
    else if (msg instanceof Blob) channel.send(msg)
    else channel.send(msg)
  }

  const sendDataControl = <T extends DataControlType>(msg: T): void => {
    const channel = controlChannel.value
    if (!channel || channel.readyState !== 'open') {
      pendingMessageControlCh.push(msg)
      return
    }

    _sendDataControl(msg)
  }

  const sendChat = <T extends DataControlType>(msg: T): void => {
    const channel = chatChannelPub.value!
    if (typeof msg === 'string') channel.send(msg)
    else if (msg instanceof Blob) channel.send(msg)
    else channel.send(msg)
  }

  const close = () => {
    controlChannel.value?.close()
    chatChannelPub.value?.close()
    chatChannelSub.value?.close()
    PublisherPeerCon.value?.close()
    SubscriberPeerCon.value?.close()

    PublisherPeerCon.value = null
    SubscriberPeerCon.value = null
    controlChannel.value = null
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
  }
}
