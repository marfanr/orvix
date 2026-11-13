export interface ScheduledMeeting {
  id: string
  alias: string
  title: string
  startAt: string
  durationMin: number
  timezone: string
  sample?: boolean
}

const storageKey = (ownerId: string) => `orvix.scheduled-meetings:${ownerId}`

function isScheduledMeeting(value: unknown): value is ScheduledMeeting {
  if (!value || typeof value !== 'object') return false
  const meeting = value as Record<string, unknown>
  return (
    typeof meeting.id === 'string' &&
    typeof meeting.alias === 'string' &&
    typeof meeting.title === 'string' &&
    typeof meeting.startAt === 'string' &&
    !Number.isNaN(Date.parse(meeting.startAt)) &&
    typeof meeting.durationMin === 'number' &&
    meeting.durationMin > 0 &&
    typeof meeting.timezone === 'string'
  )
}

export function meetingEndsAt(meeting: ScheduledMeeting) {
  return new Date(meeting.startAt).getTime() + meeting.durationMin * 60_000
}

export function listScheduledMeetings(ownerId: string): ScheduledMeeting[] {
  if (!ownerId) return []

  try {
    const raw = localStorage.getItem(storageKey(ownerId))
    if (!raw) return []

    const parsed: unknown = JSON.parse(raw)
    if (!Array.isArray(parsed)) return []

    const meetings = parsed.filter(isScheduledMeeting)
    const upcoming = meetings
      .filter((meeting) => meetingEndsAt(meeting) > Date.now())
      .sort((a, b) => Date.parse(a.startAt) - Date.parse(b.startAt))

    if (upcoming.length !== meetings.length) {
      try {
        localStorage.setItem(storageKey(ownerId), JSON.stringify(upcoming))
      } catch {
        return upcoming
      }
    }

    return upcoming
  } catch {
    return []
  }
}

export function saveScheduledMeeting(ownerId: string, meeting: ScheduledMeeting) {
  if (!ownerId) return

  try {
    const meetings = listScheduledMeetings(ownerId).filter((item) => item.id !== meeting.id)
    meetings.push(meeting)
    meetings.sort((a, b) => Date.parse(a.startAt) - Date.parse(b.startAt))
    localStorage.setItem(storageKey(ownerId), JSON.stringify(meetings))
  } catch {
    return
  }
}

export function removeScheduledMeeting(ownerId: string, id: string) {
  if (!ownerId) return

  try {
    const meetings = listScheduledMeetings(ownerId).filter((meeting) => meeting.id !== id)
    localStorage.setItem(storageKey(ownerId), JSON.stringify(meetings))
  } catch {
    return
  }
}
