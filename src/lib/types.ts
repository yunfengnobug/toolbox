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

export interface SensorInfo {
  name: string
  temperature: number
}

export interface FanInfo {
  id: string
  name: string
  rpm: number | null
  percent: number | null
  controllable: boolean
  source: string
}

export interface GpuInfo {
  name: string
  temperature: number | null
  fan_percent: number | null
  usage: number | null
  power_watt: number | null
}

export interface CoolingState {
  mode: string
  policy: string
  processor_min: number | null
  processor_max: number | null
  boost: number | null
}

export interface ThermalStatus {
  cpu_name: string
  cpu_usage: number
  cpu_temp: number | null
  memory_used: number
  memory_total: number
  sensors: SensorInfo[]
  fans: FanInfo[]
  gpus: GpuInfo[]
  cooling: CoolingState
  portable: boolean
  modes_available: boolean
  notes: string[]
}
