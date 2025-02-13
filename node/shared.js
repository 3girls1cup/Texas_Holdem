import { SecretNetworkClient, Wallet, MsgExecuteContract } from "secretjs";
import * as fs from "fs";
export const loadContractInfo = (contractInfoPath) => {
  let contractInfo;
  
  if (fs.existsSync(contractInfoPath)) {
    const contractInfoData = fs.readFileSync(contractInfoPath, "utf8");
    contractInfo = JSON.parse(contractInfoData);
    console.log("Contract info loaded:", contractInfo);
  } else {
    console.error("Contract info file not found:", contractInfoPath);
  }
  return contractInfo;
};

export const contractInfo = loadContractInfo("contractInfo.json");

export const wallet2 = new Wallet("desk pigeon hammer sleep only mistake stool december offer patrol once vacant");
export const wallet = new Wallet("pigeon desk hammer sleep only mistake stool december offer patrol once vacant");
export const wallet3 = new Wallet("hammer desk pigeon sleep only mistake stool december offer patrol once vacant");

const createSecretNetworkClient = (wallet) => {
  return new SecretNetworkClient({
    chainId: "pulsar-3",
    url: "https://pulsar.lcd.secretnodes.com",
    wallet: wallet,
    walletAddress: wallet.address,
  });
};

export const client1 = createSecretNetworkClient(wallet);
export const client2 = createSecretNetworkClient(wallet2);
export const client3 = createSecretNetworkClient(wallet3);

export const start_game = {
    start_game: {
      table_id: 42,
      players: [
        wallet.address,
        wallet2.address,
        wallet3.address,
      ],
    },
  };

export const flop = {
    community_cards: {
      table_id: 42,
      game_state: "flop", // "flop" corresponds to 1
    },
  };  

  export const turn = {
    community_cards: {
      table_id: 42,
      game_state: "turn", // "flop" corresponds to 1
    },
  };

  export const river = {
    community_cards: {
      table_id: 42,
      game_state: "river", // "flop" corresponds to 1
    },
  };

  export const showdown_all_in = {
    showdown: {
      table_id: 42,
      show_cards: [
        wallet.address,
        wallet2.address,
        wallet3.address,
      ],
      all_in_showdown: true,
    }
  };

    export const showdown = {
        showdown: {
        table_id: 42,
        show_cards: [
            wallet.address,
            wallet2.address,
            wallet3.address,
        ],
        all_in_showdown: false,
        }
    };

export const execute = async (address, secretjs, info, msg) => {
  try {
    const flip_tx = await secretjs.tx.compute.executeContract(
      {
        sender: address,
        contract_address: info.contractAddress,
        msg: msg,
        code_hash: info.contractCodeHash,
      },
      { gasLimit: 50_000 }
    );
    console.log(flip_tx);
  } catch (error) {
    console.error("Error executing contract:", error);
  }
};

export const trx = (address, info, msg) => {
  let trx = new MsgExecuteContract({
    sender: address,
    contract_address: info.contractAddress,
    msg: msg,
    code_hash: info.contractCodeHash,
  });

  return trx;
};