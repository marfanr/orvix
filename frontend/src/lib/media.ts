export function mediaErrorMessage(error: unknown, kind: 'camera' | 'microphone') {
  const label = kind === 'camera' ? 'Camera' : 'Microphone'

  if (error instanceof DOMException) {
    if (error.name === 'NotAllowedError') return `${label} permission was denied.`
    if (error.name === 'NotFoundError') return `No ${kind} was found.`
    if (error.name === 'NotReadableError') return `${label} is already in use by another app.`
  }

  return `Unable to access the ${kind}.`
}
