import { ref } from 'vue'
import { SupportedVideoCodec } from '@/types/video-call.ts'
import { useMeetingConfigStore } from '@/stores/meetingConfig'

export type DataControlType = string | Blob | ArrayBuffer

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

  const pendingCandidates: RTCIceCandidateInit[] = []
  const addedTracks = new Set<MediaStreamTrack>()
  let remoteDescriptionSet = false

  const flushPendingCandidates = async () => {
    while (pendingCandidates.length > 0) {
      const candidate = pendingCandidates.shift()
      if (!candidate) continue
      try {
        await PublisherPeerCon.value?.addIceCandidate(candidate)
      } catch (err) {
        console.warn('addIceCandidate failed', err)
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

    controlChannel.value ??= pub_pc.createDataChannel('control')
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

    chatChannelSub.value!.onmessage = (msg) => {
      const msg_data = JSON.parse(msg.data)
      console.log("found new msg: ", msg.data, msg.timeStamp)
      handleIncomingChat!(msg_data.msg, msg_data.from, msg_data.time)
    }

    const isIPv6 = (c: RTCIceCandidate) => {
      const addr = c.address ?? c.candidate.split(' ')[4] // fallback parse dari string
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
      console.log('ICE:', pub_pc.iceConnectionState)
    })
    sub_pc.addEventListener('iceconnectionstatechange', () => {
      console.log('ICE:', sub_pc.iceConnectionState)
    })

    const caps = RTCRtpReceiver.getCapabilities('video')
    // TODO: send capability to server
    console.log('capability video ', caps)

    sub_pc.addEventListener('track', (e) => {
      if (remoteVideoRef.value)
        remoteVideoRef.value.srcObject = e.streams[0] ?? new MediaStream([e.track])
      onTrack?.(e)
    })
    pub_pc.addEventListener('datachannel', (e) => {
      const channel = e.channel

      channel.onmessage = (msg) => {
        handleControl!(msg)
      }
    })
  }

  /// attach local media tracks, skipped when the track is already attached
  const addLocalStream = async (stream: MediaStream) => {
    const pc = PublisherPeerCon.value
    if (!pc) return
    for (const track of stream.getTracks()) {
      console.log('track', track)

      if (track.kind == 'video') {
        if (!localTracksTranceiver.value) {
          localTracksTranceiver.value = pc.addTransceiver(track, {
            direction: 'sendonly',
            streams: [stream],
            sendEncodings: [
              {
                rid: 'q',
                scaleResolutionDownBy: 3,
                maxBitrate: 500_000,
                maxFramerate: 30,
              },
              {
                rid: 'h',
                scaleResolutionDownBy: 1.5,
                maxBitrate: 1_500_000,
                maxFramerate: 30,
              },
              {
                rid: 'f',
                scaleResolutionDownBy: 1,
                maxBitrate: 2_500_000,
                maxFramerate: 30,
              },
            ],
          })

          const sender = localTracksTranceiver.value?.sender
          console.log('encodings after answer', sender?.getParameters().encodings)
        } else {
          await localTracksTranceiver.value.sender.replaceTrack(track)
        }
      } else if (track.kind == 'audio') {
        if (!localTracksAudio.value) localTracksAudio.value = pc.addTrack(track, stream)
        else await localTracksAudio.value.replaceTrack(track)
      }

      if (addedTracks.has(track)) continue
      addedTracks.add(track)
    }
  }

  const preferCodec = (
    pc: RTCPeerConnection,
    codec: SupportedVideoCodec,
    kind: string = 'video',
  ): void => {
    codec ??= SupportedVideoCodec.VP9
    console.log(`sellected = ${kind}/${codec.toString()}`)
    if (typeof RTCRtpSender.getCapabilities !== 'function') return

    const codecs = RTCRtpSender.getCapabilities(kind)?.codecs ?? []

    const sellectedCodec = codecs.filter(
      (c) => c.mimeType.toLowerCase() === `${kind}/${codec.toString().toLowerCase()}`,
    )
    if (sellectedCodec.length === 0) {
      console.warn(`${kind}/${codec.toString()} is not available, recording will not work`)
      return
    }

    let maxBitrate = 2_000_000
      switch (form.maxBitrate) {
        case 'auto':
          maxBitrate = 2_000_000
          break
        case '1':
          maxBitrate = 1_000_000
          break
        case '2':
          maxBitrate = 2_000_000
          break
        case '4':
          maxBitrate = 4_000_000
          break
        case '8':
          maxBitrate = 8_000_000
          break
      }
    for (const s of pc.getSenders()) {
      const p = s.getParameters()
      if (p.encodings.length == 0) continue
      for (let i = 0; i < p.encodings.length; i++) {
        // p.encodings[i]!.maxBitrate = maxBitrate
        p.encodings[i]!.networkPriority = form.networkPriority
      }
      s.setParameters(p).then()
    }

    for (const transceiver of pc.getTransceivers()) {
      const kind = transceiver.sender.track?.kind ?? transceiver.receiver.track?.kind
      if (kind !== 'video') continue

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
    preferCodec(pc, form.preferredCodec, 'video')
    const offer = await pc.createOffer()
    await pc.setLocalDescription(offer)
    return pc.localDescription?.sdp ?? offer.sdp
  }

  const setRemoteAnswer = async (sdp: string) => {
    const pc = PublisherPeerCon.value
    if (!pc) return
    await pc.setRemoteDescription({ type: 'answer', sdp })
    remoteDescriptionSet = true
    await flushPendingCandidates()
  }

  const handleRemoteOffer = async (sdp: string): Promise<string | undefined> => {
    const pc = SubscriberPeerCon.value
    if (!pc) return undefined
    if (pc.signalingState !== 'stable') {
      console.warn('handleRemoteOffer while signalingState is', pc.signalingState)
      await pc.setLocalDescription({ type: 'rollback' })
      // return undefined
    }

    await pc.setRemoteDescription({ type: 'offer', sdp })
    remoteDescriptionSet = true
    await flushPendingCandidates()
    const answer = await pc.createAnswer()
    await pc.setLocalDescription(answer)
    return pc.localDescription?.sdp ?? answer.sdp
  }

  /// candidates may arrive before the remote description is applied, buffer them
  const addIceCandidate = async (peer_type: string, candidate: RTCIceCandidateInit) => {
    const pc = peer_type == 'PUBLISHER' ? SubscriberPeerCon.value : PublisherPeerCon.value

    if (!remoteDescriptionSet) {
      pendingCandidates.push(candidate)
      return
    }
    try {
      await pc?.addIceCandidate(candidate)
    } catch (err) {
      console.warn('addIceCandidate failed', err)
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
    if (!channel || channel!.readyState !== 'open') {
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
    PublisherPeerCon.value?.close()

    PublisherPeerCon.value = null
    controlChannel.value = null

    localTracksVideo.value = null
    localTracksAudio.value = null
    localTracksTranceiver.value = null

    pendingCandidates.length = 0
    pendingMessageControlCh.length = 0

    addedTracks.clear()
    remoteDescriptionSet = false
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
