#!/bin/bash

sudo dpkg -i /tmp/concordium-node*

sudo mv /usr/bin/concordium-mainnet-node-6.0.4 /usr/bin/concordium-node
sed -i 's/\concordium-mainnet-node-[^ ]*/concordium-node' /lib/systemd/system/concordium-mainnet-node.service

sudo systemctl daemon-reload
sudo systemctl enable --now concordium-mainnet-node.service
sudo systemctl enable --now concordium-mainnet-node-collector.service