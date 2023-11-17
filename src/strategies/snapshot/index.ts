import fetch from 'cross-fetch';
import { BigNumber } from '@ethersproject/bignumber';

const SNAPSHOT_HUB_URL = 'https://hub.snapshot.org/graphql';
const SNAPSHOT_TESTNET_HUB_URL = 'https://testnet.hub.snapshot.org/graphql';

export default async function strategy(recipient: string, params: any): Promise<string> {
  const url = params.env === 'testnet' ? SNAPSHOT_TESTNET_HUB_URL : SNAPSHOT_HUB_URL;

  const init = {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json'
    },
    body: JSON.stringify({
      query: `
        query Vote($voter: String!, $proposal: String!) {
          votes(
            first: 1
            where: {
              voter: $voter
              proposal: $proposal
            }
          ) {
            vp
            vp_state
          }
        }
      `,
      variables: {
        voter: recipient,
        proposal: params.proposal
      }
    })
  };
  const res = await fetch(url, init);
  const { data } = await res.json();

  if (!data.votes[0] || data.votes[0].vp === 0) return '0';

  if (params.type === 'ratio')
    return BigNumber.from(params.amount).mul(BigNumber.from(data.votes[0].vp)).toString();

  if (params.type === 'fixed') return params.amount;

  return '0';
}
