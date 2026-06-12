export class InvoiceService {
  applyTax(value: number): number {
    return Math.round(value * 1.1);
  }
}
