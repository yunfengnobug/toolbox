import { invoke } from '@tauri-apps/api/core'
import type { IdentifierInfo } from './types'

export async function getPlatform(): Promise<string> {
  return invoke<string>('get_platform')
}

export async function checkAdmin(): Promise<boolean> {
  return invoke<boolean>('check_admin')
}

export async function getAllIdentifiers(): Promise<IdentifierInfo[]> {
  return invoke<IdentifierInfo[]>('get_all_identifiers')
}

export async function modifyIdentifier(id: string, newValue: string): Promise<void> {
  return invoke('modify_identifier', { id, newValue })
}

export async function generateRandomValue(id: string): Promise<string> {
  return invoke<string>('generate_random_value', { id })
}

export async function backupIdentifiers(): Promise<string> {
  return invoke<string>('backup_identifiers')
}

export async function restoreIdentifiers(): Promise<void> {
  return invoke('restore_identifiers')
}
