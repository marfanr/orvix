export function userIdFromToken(token: string | null | undefined): string {
  if (!token) return ''

  try {
    const base64 = (token.split('.')[1] ?? '').replace(/-/g, '+').replace(/_/g, '/')
    const payload = JSON.parse(atob(base64))
    return String(payload.sub ?? '')
  } catch {
    return ''
  }
}
