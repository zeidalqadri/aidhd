export const MM_PER_INCH = 25.4
export const PT_PER_INCH = 72

export type LengthUnit = 'mm' | 'in'

export const mmToInch = (mm: number) => mm / MM_PER_INCH
export const inchToMm = (inch: number) => inch * MM_PER_INCH
export const inchToPt = (inch: number) => inch * PT_PER_INCH
export const mmToPt = (mm: number) => mmToInch(mm) * PT_PER_INCH

export function toInches(value: number, unit: LengthUnit): number {
  return unit === 'mm' ? mmToInch(value) : value
}

/** Effective print resolution when `pixels` are printed across `inches`. */
export function effectiveDpi(pixels: number, inches: number): number {
  if (inches <= 0) return 0
  return pixels / inches
}

export const MIN_PRINT_DPI = 300

export function isDpiAdequate(pixels: number, inches: number, min = MIN_PRINT_DPI): boolean {
  return effectiveDpi(pixels, inches) >= min
}

export interface SheetSpec {
  name: string
  widthPt: number
  heightPt: number
}

export const SHEETS: Record<'A4' | 'Letter', SheetSpec> = {
  A4: { name: 'A4', widthPt: mmToPt(210), heightPt: mmToPt(297) },
  Letter: { name: 'Letter', widthPt: inchToPt(8.5), heightPt: inchToPt(11) },
}
