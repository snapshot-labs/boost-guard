import snapshot from '@snapshot-labs/snapshot.js';
import { JsonRpcProvider } from '@ethersproject/providers';

const { Multicaller } = snapshot.utils;

export async function getToken(token: string, chainId: number) {
  const abi = [
    'function name() view returns (string)',
    'function symbol() view returns (string)',
    'function decimals() view returns (uint8)'
  ];
  const provider = new JsonRpcProvider(`https://rpc.snapshot.org/${chainId.toString()}`);
  const multi = new Multicaller(chainId.toString(), provider, abi, {});
  multi.call('name', token, 'name');
  multi.call('symbol', token, 'symbol');
  multi.call('decimals', token, 'decimals');
  return await multi.execute();
}

export async function sleep(time) {
  return new Promise(resolve => setTimeout(resolve, time));
}
