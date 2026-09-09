import { invoke } from '@tauri-apps/api/core'
import type { IdentifierInfo, ThermalStatus } from './types'

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

export async function getThermalStatus(): Promise<ThermalStatus> {
  return invoke<ThermalStatus>('get_thermal_status')
}

export async function setCoolingMode(mode: string): Promise<string> {
  return invoke<string>('set_cooling_mode', { mode })
}

export async function setFanPercent(fanId: string, percent: number): Promise<string> {
  return invoke<string>('set_fan_percent', { fanId, percent })
}
