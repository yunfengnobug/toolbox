export interface IdentifierInfo {
  id: string
  label: string
  value: string
  modifiable: boolean
  description: string
}

export interface ToastMessage {
  id: number
  message: string
  type: 'success' | 'error' | 'info'
}
