import express from 'express';
import { Wallet } from '@ethersproject/wallet';
import { rpcError, rpcSuccess } from './utils';
import { Guard } from './guard';

const privateKey = process.env.GUARD_PK || '';
const wallet = new Wallet(privateKey);
const guard = new Guard(wallet);

const router = express.Router();

router.post('/', async (req, res) => {
  const { id, params } = req.body;
  try {
    const { boostId, recipient } = params;
    const result = await guard.run(boostId, recipient);
    const claim = await guard.claim(result);
    return rpcSuccess(res, claim, id);
  } catch (e) {
    return rpcError(res, 500, e, id);
  }
});

export default router;
