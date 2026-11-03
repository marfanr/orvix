export interface Participant {
  id: number
  conn_id: string | undefined
  name: string
  muted: boolean
  cameraOff: boolean
  stream: MediaStream | null
}

export interface JoinRequest {
  username: string
  conn_id: string
}

export interface FloatingReaction {
  key: number
  emoji: string
  left: number
}

export type RoomStatus = 'loading' | 'joined' | 'denied' | 'waiting' | 'not-started' | 'not-found'

export type MemberChangesType = 'join' | 'leave'

export enum PopUpWidth {
  MEDIUM,
  LARGE,
}

export interface MemberChanges {
  eventId: number
  username: string
  type: MemberChangesType
}

export interface ChatMessage {
  id: number
  sender: string
  text: string
  time: string
  outgoing: boolean
}

export enum SupportedVideoCodec {
  H264 = 'H264',
  VP8 = 'VP8',
  VP9 = 'VP9',
  AV1 = 'AVI'
}

export enum SupportedBitrate {
  AUTO = 'auto',
  B1 = '1',
  B2 = '2',
  B4 = '4',
  B8 = '8',
}