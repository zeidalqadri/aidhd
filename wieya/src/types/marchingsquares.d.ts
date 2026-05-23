declare module 'marchingsquares' {
  /** Each contour is an array of [x, y] points; first === last (closed). */
  export function isoContours(
    data: number[][],
    threshold: number,
    options?: Record<string, unknown>,
  ): number[][][]
  export function isoLines(
    data: number[][],
    threshold: number,
    options?: Record<string, unknown>,
  ): number[][][]
  export function isoBands(
    data: number[][],
    lowerBound: number,
    bandWidth: number,
    options?: Record<string, unknown>,
  ): number[][][]
}
