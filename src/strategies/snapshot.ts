import Strategy from './strategy';
import snapshot from '@snapshot-labs/snapshot.js';

const HUB_URL = 'https://hub.snapshot.org/graphql';

export default class Snapshot extends Strategy {
  async run() {
    const result = await snapshot.utils.subgraphRequest(HUB_URL, {
      votes: {
        __args: {
          first: 1,
          where: {
            voter: this.recipient,
            proposal: this.tag
          }
        },
        vp: true,
        vp_state: true
      }
    });
    console.log('Vote', result);

    // if (result.votes[0].vp_state === 'pending') return Promise.reject('vp state must be final');

    let amount = 123; // result.votes[0].vp;

    if (this.params.min && amount < this.params.min) amount = this.params.min;
    if (this.params.min && amount > this.params.max) amount = this.params.max;

    return {
      boostId: this.boostId,
      recipient: this.recipient,
      amount
    };
  }
}
