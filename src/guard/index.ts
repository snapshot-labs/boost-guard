import { Wallet } from '@ethersproject/wallet';
import { domain, Claim, claimTypes } from './types';

export class Guard {
  private readonly wallet: Wallet;
  public address: string;

  constructor(wallet: Wallet) {
    this.wallet = wallet;
    this.address = wallet.publicKey;
  }

  public async claim(message: Claim) {
    const data: any = { domain, types: claimTypes, message };
    const sig = await this.wallet._signTypedData(domain, claimTypes, message);
    return { address: this.address, sig, data };
  }
}
