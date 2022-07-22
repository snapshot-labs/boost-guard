import snapshot from '@snapshot-labs/snapshot.js';
import { Wallet } from '@ethersproject/wallet';
import { domain, Claim, claimTypes, Strategy } from './types';
import * as strategies from '../strategies';

const BOOST_SUBGRAPH = 'https://api.thegraph.com/subgraphs/name/mktcode/boost';

export class Guard {
  private readonly wallet: Wallet;
  public address: string;

  constructor(wallet: Wallet) {
    this.wallet = wallet;
    this.address = wallet.publicKey;
  }

  public async getMetadata(boostId): Promise<Strategy> {
    const result = await snapshot.utils.subgraphRequest(BOOST_SUBGRAPH, {
      boost: {
        __args: {
          id: boostId
        },
        strategyURI: true,
        tag: true
      }
    });
    const strategyURI = !result.boost.strategyURI.includes('://')
      ? `ipfs://${result.boost.strategyURI}`
      : result.boost.strategyURI;
    return await snapshot.utils.getJSON(snapshot.utils.getUrl(strategyURI));
  }

  public async run(boostId: string, recipient): Promise<Claim> {
    const metadata = await this.getMetadata(boostId);
    const strategy = new strategies[metadata.name].default(
      boostId,
      recipient,
      metadata.tag,
      metadata.params
    );
    return await strategy.run();
  }

  public async claim(message: Claim) {
    const data: any = { domain, types: claimTypes, message };
    const sig = await this.wallet._signTypedData(domain, claimTypes, message);
    return { address: this.address, sig, data };
  }
}
