
# **How we utilize CI in improving the performance of the Mina network**

# Table of Contents
1. [Introduction](#Introduction)
2. [CI-assisted testing and optimization](#CI-assisted-testing-and-optimization)
3. [Analyzing the data](#Analyzing-the-data)
4. [The front end](#The-Front-End)


## Introduction

Blockchains are highly secure networks, but their performance leaves much to be desired — they are not currently able to process enough transactions per second (which is also known as their _throughput_) to become widely adopted by the general public. If we want to achieve this widespread adoption, we need to _optimize_ blockchains for better performance.

  

Having completed the pre-requisite to optimization, which is [the tracing](https://medium.com/@jurajselep/utilizing-internal-and-external-tracing-of-the-mina-network-to-identify-areas-of-code-in-need-of-54656bb48f60) of the Mina network’s various metrics, we are now ready to take the next step — re-writing the code that we have identified as in need of optimization, and running continuous integration (CI) tests to confirm that our changes have resulted in better performance.

  

The next step is to rewrite the parts of the code that are particularly slow and most in need of optimization, creating new builds of the code that we can then run and test. Whenever we introduce changes to the code, it is possible that despite our intentions, there will actually be a performance regression.

  

Testing these changes promptly upon implementation allows us to identify performance regressions without delay. This immediate detection is crucial, as it enables developers to address any slowdowns or inefficiencies introduced by their modifications, ensuring that the system maintains optimal performance.

  

Since we are interested in improving the performance of the node, we focus on measuring the following processes to ensure a faster and more efficient blockchain network:

  
  
  

* **block production** — how long it took the producer node to produce the block.

  

_Faster block production enables quicker block broadcasting. By reducing the time needed for block production, we can process more transactions per second, resulting in increased throughput and a more responsive network. This influences the next two metrics as well._

  
  
  

* **broadcast broadcast** — how long it took since production time the node to receive this block.

  

_This evaluates the time elapsed between the production of a block and when the node receives it. Maximizing the efficiency of block broadcasting ensures that nodes can validate and propagate the block more quickly, leading to better overall network performance._

  
  
  

* **block application** — how long it took the node since he received this block from the network to apply this block to its chain.

  

_If a new block is received too late, potential producers might not have enough time to apply and validate the block before their scheduled block production slot. Optimizing block application times prevents potential block producers from missing their opportunity to produce new blocks, increasing the stability and efficiency of the network._

  

Although current block times are fixed, achieving faster block production, faster application time and better broadcast latency may enable the reduction of block times in future iterations. This reduction in block times would lead to increased transaction throughput and an overall more efficient and responsive blockchain infrastructure

  

Measuring these processes informs us of whether the changes to the code have helped optimize these processes (reduced the time needed for them), or if they have resulted in performance regressions.

  

In addition to evaluating the performance of the Mina Protocol under high transaction loads, we will also compare the results across multiple commits.

  

This comparative analysis helps us detect any regressions introduced during development, helping ensure that the Mina network remains stable and efficient throughout its evolution. It also helps us identify specific optimizations and enhancements introduced in each version, so that we can see which changes had the most significant positive impact on the Mina Protocol’s performance and stability. Finally, it helps guide future development, as by understanding the impacts of specific changes, we can make informed decisions when implementing new features or optimizations.

  

For these purposes, we utilize continuous integration (CI), a software development practice where developers integrate their code into a central repository (such as Github) frequently and run automated tests to detect bugs and issues as early as possible. The goal is to ensure that the codebase remains in a stable and working state, and in our case, that the new code is actually improving the network’s performance.

  
  

## **CI-assisted testing and optimization**

  

With the use of the CI, we run builds of the code in a specified frequency. We can divide these builds into 2 parts:

  
  
  

1. **Preparation**

  

We first prepare a Docker image of the Mina node that contains changes from the last build. To compare it with previous builds, we need to run multiple Docker images of the Mina node — for both the old build and the new one.

  

For this, we utilize Kubernetes, a system for managing multiple instances of containerized applications. Containers are similar in function to virtual machines but are more lightweight and efficient since they share the same OS kernel as the host.

  

We then deploy a custom test network inside a so-called Kubernetes _cluster_, a network of containers, each of which is running one of the Mina node’s Docker images. This includes plain Mina nodes, block producer nodes, seed nodes, and SNARK workers.

  

In order to ensure that the test network was deployed successfully (i.e. all nodes are synced with each other), at this stage, we also perform various so-called health checks by probing the app to see how it responds.

  

**2. Testing**

  

We stress-test the network by sending a large number of transactions to the nodes. The goal is to saturate a block with transactions — there is a limit on the number of transactions included in a block (around 128 with the current protocol), so we need to generate at least that number of zkApp transactions per block.

  

For a transaction to be included in a block, some snark work is needed in order to generate a proof for that transaction. Completed snark work is usually a bundle of two proofs, though rarely it can be a single proof (1 in 128 completed snark works include a single proof). Therefore we want to be able to produce enough completed work to sustain the production of saturated blocks.

  

While this stress test is running, we launch an aggregator inside the Kubernetes cluster, which is an application that collects and aggregates data from all of the Mina nodes inside the test network.

  

We also want to ensure that block production performance is not degraded when there is a high load on SNARK generators.

  

For that purpose, we utilize a zkApps test. In the first stage, we deploy a testnet consisting of 1 seed node, 5 block-producing nodes using 3 accounts, 64 snark workers using HTTP-based work coordinator, and 8 plain nodes. The latter are used in the following stage to inject zkApp transactions.

  

In this test, we are using a very simple zkApp that updates its on-chain state (the state that is stored in the Mina blockchain state). Here is the[ link to the repository on GitHub](https://github.com/openmina/mina-load-generator/blob/c26e6e7ad941dd096808b37f3cdf694240715cfc/src/Add.ts).

  

The aim is to generate enough zkApp transactions so that snark work would be needed to include these transactions to a block (currently 2048 transactions). In our cluster, it takes from 40 seconds to 2 mins to generate a proof for a transaction on the client side, so we use 32 jobs that do that in parallel until the required number of transactions is generated. Once all transactions have been generated, the test waits for slot #30, in order for all transactions to be included in blocks.

  
  

## **Analyzing the data**

  

An essential feature of the aggregator tool is its ability to calculate delta values between two runs of the stress test, enabling a direct comparison of performance metrics for different commits or test configurations. This delta analysis helps us identify trends in the network’s performance and assess the impact of specific changes introduced in each commit.

  

The aggregator tool also plays a crucial role in detecting regressions, which are instances where the network’s performance has degraded compared to previous versions. By analyzing the delta values, the tool can automatically determine if a regression has occurred, enabling developers to pinpoint the underlying cause and address it promptly.

  

This data is then fetched by our CI reporting front-end application in which we can view and compare multiple runs of the CI tests to see if there are regressions or speedups in performance.

  
  

### **The Front End**

  

To enhance the accessibility and visualization of the data gathered by the aggregator, we have developed a frontend webpage that displays the key performance metrics and comparative analysis across multiple commits. It allows developers and stakeholders to quickly and easily interpret the results of the stress tests, enabling informed decision-making and facilitating network improvements

  

Click here to open up the[ CI reports interface](https://perf.ci.openmina.com/) in your internet browser.

  
  

#### **Trends**

  

This tab displays an overview of the various builds at monthly intervals.

  
  
  
![CI1](https://user-images.githubusercontent.com/60480123/229048073-fd0d1096-4d75-4f19-a98b-6027bd1ef275.png)
  
  

You can also switch to weekly intervals by clicking on the **Week** button in the upper right corner of the screen.

  
  
  ![CI2](https://user-images.githubusercontent.com/60480123/229048140-da52a236-759b-4232-94fb-9d9a657c669c.png)

  
  

Each point in the graph shows the build’s **maximum** and **average** time for block production, block broadcast, and block application, in that month or week. This provides a simple overview of the progress of various builds over time.

  
  
  

![CI3](https://user-images.githubusercontent.com/60480123/229048174-cee84433-92ee-40ac-bb29-01d799a0a4dd.png)

  
  

**Block Production**  —  how long it took the producer node to produce the block

  
  
  
![CI4](https://user-images.githubusercontent.com/60480123/229048228-84aafff7-edf6-4f1c-bfc8-b0384e04b639.png)


  
  

**Block Broadcast** — how long it took since production time the node to receive this block

  
  
  ![CI5](https://user-images.githubusercontent.com/60480123/229048263-7e08a8a4-8dc8-47ec-a604-fdb13a042cd9.png)


  

**Block Application** — how long it took the node since he received this block from the network to

  

You can click on the graph to see that build’s details:

  
  
  ![CI6](https://user-images.githubusercontent.com/60480123/229048306-82f0800d-01ad-428c-b4f4-a2a93ccc8e1b.png)


  

**Build 178 — Increase the number of zkapp tx sent** —  the name of the build

  

**Open in GitHub** — Opens this build in GitHub

  

**Open in Drone** —  Opens this build in drone

  
  

##### **Environment**

  

**Nodes**— How many nodes were used in this build

  

**Snarkers** — How many snark workers were used in this build

  

**Producers** —  How many block producing nodes were used in this build

  
  

##### **Blocks**

  

**Total — 64 blocks** — how many blocks were produced in this build. Click on this to show further details for each block:

  
  
  
![CI7](https://user-images.githubusercontent.com/60480123/229048361-74dc57b8-0e95-4bbf-8854-5b9ae5f043f1.png)


  
  

**Height**— the block’s height. Note that multiple blocks may have the same height, because in Mina, multiple so-called _candidate blocks_ may be produced at the same height by different block producers.

  

**Gl. Slot**— the block’s global slot (a slot irrespective of Mina epochs)

  

**Hash** — the hash of the block

  

**Tx** — how many transactions it contains

  

**Max Lat.**— The maximal receive latency, i.e. the latest time the block arrived in a node since its creation

  

**Prod Nodes** — How many nodes produced this block

  
  

#### **Builds**

  
  
  
  ![CI8](https://user-images.githubusercontent.com/60480123/229048412-ef7f7d6c-d9a9-4a6b-a790-1c12542124f0.png)

  

This is a list of CI reports.

  
  
  
![CI9](https://user-images.githubusercontent.com/60480123/229048447-feda51a1-ff76-4f97-a1c2-458a8bff6edd.png)


  
  

At the top, there are filters. Click on the Filters icon to show a list of filters, and select one or multiple filters to show builds within those categories:

  

**Pending** — The CI build is in pending status, this means the build has yet to start.

  

**Running** — The CI build is currently running.

  

**Success** — The CI build has been completed and all the tests completed successfully.

  

**Failure** — The CI build has been completed and some of the tests failed, so we have no reliable values to display.

  

**Killed** — The CI build was killed, whether manually, or by the Drone CI (for example, it took too long to run).

  

In the upper right corner of the screen, there are several buttons:

  

**Refetch**— updates the status of data manually, otherwise, it is updated periodically every X amount of seconds

  

**Show deltas**— compares the most recent build and the previous one. It does so by changing the histograms to show deltas.

  
  
  
![CI10](https://user-images.githubusercontent.com/60480123/229048493-683bf2e8-2078-4d94-bf9b-8cd7bf7df324.png)


  
  

**Decrease the number of jobs** — the title of the commit, which describes the changes made in the build

  

**f976ab7** — the hash of the commit. Clicking on it will open up the commit on GitHub.

  

**to openmina-berkeley** — Which branch the commit was pushed to.

  

**11 hours ago 00:54:24 16 Mar 2023** — date and time when it was made.

  

**Killed**— The status of the build. Clicking on this will open up the run in Drone, our CI application.

  

**No Verdict** — whether it has improved the performance or not. _Regression_ means the performance has regressed, _Passed_ means performance was improved, _No Verdict_ means it cannot be determined.

  

**Transactions** — Number of transactions processed in this CI run (47)

  

**Blocks** — The block height reached in this CI run (4)

  
  

##### **Histograms**

  
  
  

![CI11](https://user-images.githubusercontent.com/60480123/229048538-43fed01f-aa0b-4443-98cb-24783155ba31.png)

  
  

There are three histograms, representing the following:

  

**Block Production** — how long it took the producer node to produce the block.

  

**Block** **Broadcast** — how long it took since production time the node to receive this block.

  

**Block Application** — how long it took the node since he received this block from the network to apply this block to its chain.

  

Histograms can be changed to show the differences between the most recent build and the previous build by clicking on the **Show deltas** button in the upper right corner of the screen:

  
  
  
![CI12](https://user-images.githubusercontent.com/60480123/229048579-4449aea6-03ea-4eca-9b9a-1d70c72df8d2.png)


  
  

This displays a histogram in which you can see the difference in each value between two builds. If the value has increased, the bar will be in red and will be rising up, if it has decreased, the bar will be in green and going down.

  

Now click on a report to show more details:

  
  
  
![CI13](https://user-images.githubusercontent.com/60480123/229048616-686108ba-0d47-4d0f-a788-fec98be8a846.png)


  
  

On the right side of your screen, a sidebar with additional Report Details will show up with the production, application, and latency histograms for that build.

  

At the top is the number and title (name) of the build.

  

Below the title are links through which you can examine the build in GitHub and Drone.

  

You can also **compare** this build with other builds. Clicking on this button will take you to the compare tab, with this build already selected as one of the two that are being compared.

  

Back on the build tab, further down you can see the **Total** number of blocks produced with this build. Click on this to open up details of these blocks:

  
  
  
![CI14](https://user-images.githubusercontent.com/60480123/229048647-0384d13b-3b81-401d-83d3-6c3af3b3bea0.png)


  
  

**Height** — the height of the block

  

**Gl. Slot** — Global slot (a slot irrespective of Mina epochs)

  

**Hash** — the hash of the block

  

**Tx** — how many transactions it contains

  

**Max Lat.**— The maximal receive latency, i.e. the latest time the block arrived in a node since its creation

  

**Prod Nodes** — How many nodes was this block produced by

  

You can click on any of the aforementioned values to sort blocks by that value.

  
  

#### **Compare**

  

On this tab, you can compare CI reports from two different builds.

  

Click on a build’s number to show a drop-down list of various builds to select from:

  
  
  
![CI15](https://user-images.githubusercontent.com/60480123/229048692-dd7c0fbd-2131-41ac-842f-6080235fb891.png)


  
  

In the upper left part of the screen, you can select the two builds you want to compare. Between these two builds, you can see whether there was a _regression_ or it has _passed_, meaning there was an improvement in performance.

  
  
  
![CI16](https://user-images.githubusercontent.com/60480123/229048738-92bb4640-6367-4168-afbc-75237d345df4.png)


  
  

Below the two builds are their deltas for three key metrics: _block production_, _block broadcasting_ and _block application_.

  

On the right, you can see further details for each build:

  

**Environment** — what kind of environment the build was run in.

  

**Nodes** — how many nodes were used in this run.

  

**Snarkers** — how many nodes running Snark workers were used in this run.

  

**Producers** — how many block producers were used in this run.

  
  

#### **Blocks**

  

**Total** — the total amount of blocks that were produced in this run.

  

There are three histograms representing the following:

  

**Block Production** times — how long it took the producer node to produce the block.

  

**Block Broadcast** — how long it took for the node to receive this block since production time.

  

**Block Application** — how long it took the node since he received this block from the network to apply this block to its chain times

  

In the future, we plan on adding network split testing, through which we will test how the network manages to handle short and long forks. We also want to use different kinds of zkApps to generate transactions, including multi-account transfers, off-chain state updates, and contracts with more sophisticated code that result in bigger inputs for proof generation

  

We thank you for taking the time to read this article. For a more detailed look at the Tracing and Metrics interface, check out our [guide on GitHub](https://github.com/openmina/mina-frontend/blob/main/docs/MetricsTracing.md). If you have any comments, suggestions or questions, feel free to contact me directly by email. To read more about OpenMina and the Mina Web Node, subscribe to our[ Medium](https://medium.com/openmina) or visit our[ GitHub](https://github.com/openmina/openmina).


