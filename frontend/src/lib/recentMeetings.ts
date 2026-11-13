export interface RecentMeeting {
  alias: string
  title: string
  lastOpenedAt: string
  sample?: boolean
}

const storageKey = (ownerId: string) => `orvix.recent-meetings:${ownerId}`
const MAX_RECENT = 8

function isRecentMeeting(value: unknown): value is RecentMeeting {
  if (!value || typeof value !== 'object') return false
  const meeting = value as Record<string, unknown>
  return (
    typeof meeting.alias === 'string' &&
    typeof meeting.title === 'string' &&
    typeof meeting.lastOpenedAt === 'string' &&
    !Number.isNaN(Date.parse(meeting.lastOpenedAt))
  )
}

export function listRecentMeetings(ownerId: string): RecentMeeting[] {
  if (!ownerId) return []

  try {
    const raw = localStorage.getItem(storageKey(ownerId))
    if (!raw) return []

    const parsed: unknown = JSON.parse(raw)
    if (!Array.isArray(parsed)) return []

    return parsed
      .filter(isRecentMeeting)
      .sort((a, b) => Date.parse(b.lastOpenedAt) - Date.parse(a.lastOpenedAt))
      .slice(0, MAX_RECENT)
  } catch {
    return []
  }
}

export function recordRecentMeeting(
  ownerId: string,
  meeting: Pick<RecentMeeting, 'alias' | 'title'>,
) {
  if (!ownerId || !meeting.alias) return

  try {
    const meetings = listRecentMeetings(ownerId).filter((item) => item.alias !== meeting.alias)
    meetings.unshift({
      alias: meeting.alias,
      title: meeting.title || 'Untitled meeting',
      lastOpenedAt: new Date().toISOString(),
    })
    localStorage.setItem(storageKey(ownerId), JSON.stringify(meetings.slice(0, MAX_RECENT)))
  } catch {
    return
  }
}

export function removeRecentMeeting(ownerId: string, alias: string) {
  if (!ownerId) return

  try {
    const meetings = listRecentMeetings(ownerId).filter((meeting) => meeting.alias !== alias)
    localStorage.setItem(storageKey(ownerId), JSON.stringify(meetings))
  } catch {
    return
  }
}
