## How to start a CI run

1. Go to https://ci.openmina.com/
2. Log in using github credentials
3. (You can skip the Drone registration by submitting an empty form)
4. Navigate to the openmina/mina repository
5. Click on NEW BUILD button in the top right corner
6. Into the Branch field type in `openmina-berkeley`. This is needs to be set to the branch that includes all modified helm charts and our .drone.yml file, which is the configuration file for the CI builds.
7. (Optional) You can specify parameters (see bellow) as key-value pairs to customize the build
8. Click on Create

### Customization

If you do not specify any additional Parameters, the build will run with it's default parameters.

Avalable parameters:

| Parameter      | Description                              | Default value |
| -------------- | ---------------------------------------- | ------------- |
| MINA_REPO      | Link to the mina repository              | https://github.com/openmina/mina |
| MINA_BRANCH    | The branch to create the mina image from | openmina-berkeley |
| NAMESPACE      | The namespace to deploy our network to   | testnet-default |

Notes:

- For now in the `Branch` field, you have to specify `openmina-berkeley`, i.e. the branch we have all our modified helm charts in. To pick a branch to build the image from, you have to specify `MINA_BRANCH`. If you want to build from a branch that is in a different repository (e.x.: MinaProtocol/mina), you have to specify two parameters `MINA_REPO` and `MINA_BRANCH`. 

- The namespace has to be prepared beforhand to include all the resources (like secrets). Currently we have only one namespace available: `testnet-default`.

### Testnet network specification

- Seed nodes: 1
- Plain nodes: 8
- Producer nodes: 5
- Snark worker nodes: 64

You can check out the configuration in the helm charts at: https://github.com/openmina/mina/blob/openmina-berkeley/helm/openmina-config/values/common.yaml

<!-- Here is a gif that depicts the steps above: -->

<!-- ![How to start a CI run](/docs/assets/create-custom-build-2.gif) -->

<!-- openmina-berkeley -->