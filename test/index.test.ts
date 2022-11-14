import strategies from '../src/strategies';

describe('Strategies', () => {
  it('snapshot', async () => {
    const recipient = '0xeF8305E140ac520225DAf050e2f71d5fBcC543e7';
    const params = {
      proposal: '0x0021dc765768342d269184cb46e1cf17e6609559973d88bd01a292a4f390caa4',
      type: 'fixed',
      amount: '1000'
    };

    const status = await strategies.snapshot(recipient, params);

    expect(status).toMatchSnapshot();
  });
});
