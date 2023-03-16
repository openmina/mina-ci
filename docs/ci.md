
# How we utilize CI in improving the performance of the Mina network

Blockchains are highly secure networks, but their performance leaves much to be desired - they are not currently able to process enough transactions per second (which is also known as their _throughput_) to become widely adopted by the general public. If we want to achieve this widespread adoption, we need to _optimize_ blockchains for better performance.

Having completed the pre-requisite to optimization, which is the tracing of the Mina network’s various metrics, we are now ready to take the next step - re-writing the code that we have identified as in need of optimization, and running continuous integration (CI) tests to confirm that our changes have resulted in better performance.


### CI implementation

The next step is to rewrite the parts of the code that are particularly slow and most in need of optimization, creating new builds of the code that we can then run and test. 

Testing ensures that the updated code does, in fact, improve the Mina network’s performance. For this, we utilize continuous integration (CI), a software development practice where developers integrate their code into a central repository (such as Github) frequently and run automated tests to detect bugs and issues as early as possible. The goal is to ensure that the codebase remains in a stable and working state, and in our case, that the new code is actually improving the network’s performance.


### How it works

With the use of the CI, we run builds of the code in a specified frequency. We can divide these builds into 2 parts: 



1. **Preparation** 

    We first prepare a Docker image of the Mina node that contains changes from the last build. To compare it with previous builds, we need to run multiple Docker images of the Mina node - for both the old builds and the new - for which we utilize Kubernetes, a system for managing multiple instances of containerized applications. 


    We then deploy a custom test network inside a so-called Kubernetes _cluster_, a network of containers similar to virtual machines, each of which is running one of the Mina node’s Docker images. This includes plain Mina nodes, block producer nodes, seed nodes, and SNARK workers. In order to ensure that the test network was deployed successfully (i.e. all nodes are synced with each other), at this stage, we also perform various so-called health checks by probing the app to see how it responds. 

2. **Testing** 

    We stress-test the network by sending a large amount of transactions to the nodes. While this stress test is running, we launch an aggregator inside the Kubernetes cluster, which is an application that collects and aggregates data from all of the Mina nodes inside the test network.


    Since we are interested in improving the performance of the node, the following categories of data are collected and processed by the aggregator: 

* block production times (how long it took the producer node to produce the block)
* block application (how long it took the node since he received this block from the network to apply this block to its chain) times
* receive latencies (how long it took since production time the node to receive this block). 

    This data is then fetched by our CI reporting front-end application in which we can view and compare multiple runs of the CI tests to see if there are regressions or speedups in performance. 



### The CI reports interface

Open up the [CI reports interface](http://1.k8.openmina.com:31356/140) in your internet browser.

**Reports**

![0-ListOfReports](https://user-images.githubusercontent.com/60480123/225696180-167feccb-f43d-43a7-9635-7e1268b99481.png)


This is a list of CI reports.

![1-1-FiltersDetail](https://user-images.githubusercontent.com/60480123/225695997-df936d54-4ce0-4cf0-9d5d-1907f92a3dfa.png)


At the top are filters. Click on the Filters icon to show a list of filters, and select one or multiple filters to show builds within those categories:

**Pending** - The CI build is in pending status, this means the build has yet to start.

**Running** - The CI build is currently running.

**Success** - The CI build has completed and all the tests completed successfully.

**Failure** - The CI build has completed and some of the tests failed, so we have no reliable values to display.

**Killed** - The CI build was killed wither manually, or by the Drone CI (for example, it took too long to run). 

In the upper right corner of the screen there are several buttons:

**Refetch** - updates the status of data manually, otherwise, it is updated periodically every X 

amount of seconds

**Show deltas** - compares the most recent build and the previous one. It does so by changing the histograms to show deltas.


![0-1-ListOfReportsDetail](https://user-images.githubusercontent.com/60480123/225696365-26ede229-6415-440c-a6af-7cd376e3e2a3.png)


**Decrease the number of jobs** - the title of the commit, which describes the changes made in the build

**f976ab7** - the hash of the commit. Clicking on it will open up the commit on GitHub.

**to openmina-berkeley** -  

**11 hours ago 00:54:24 16 Mar 2023** - date and time when it was made

**Killed** - The status of the build. Clicking on this will open up the run in Drone, our CI application.

**No Verdict** - whether it has improved the performance or not. Regression means the performance has regressed, Passed means performance was improved, No Verdict means it cannot be determined.

**Transactions** - Number of transactions processed in this CI run (47)

**Blocks** - The block height reached in this CI run (4)


#### Histograms

![Histograms](https://user-images.githubusercontent.com/60480123/225696472-c43925de-d517-46d4-a7cc-7df8a268082d.png)


There are three histogram representing the following:

Block **Production** times - how long it took the producer node to produce the block

Block **Application** - how long it took the node since he received this block from the network to apply this block to its chain times

Receive **Latency** - how long it took since production time the node to receive this block

Histograms can be changed to show the differences between the most recent build and the previous build by clicking on the **Show deltas** button in the upper right corner of the screen:


![Deltas](https://user-images.githubusercontent.com/60480123/225696579-8bf23127-1977-4b20-9605-764be381cb16.png)


This displays a histogram in which you can see the difference in each value between two builds. If the value has increased, the bar will be in red and will be rising up, if it has decreased, the bar will be in green and going down.

Now click on a report to show more details:


![3-Sidebar](https://user-images.githubusercontent.com/60480123/225696674-7c21e118-1378-40b9-9f72-3440bd9eeba0.png)


On the right side of your screen, a side bar with additional Report Details will show up with the production, application and latency histograms for that build.

Above the histograms, you can see the **Total** number of blocks produced with this build. Click on this to open up details of these blocks:


![3-1-Sidebar AllBlocks](https://user-images.githubusercontent.com/60480123/225697418-443d6b50-bcee-4185-91b9-2d201770a08c.png)



**Height** - the height of the block

**Gl. Slot** - Global slot (a slot irrespective of Mina epochs)

**Hash** - the hash of the block

**Tx** - how many transactions it contains

**Max Lat.** - The maximal receive latency, i.e. the latest time the block arrived in a node since its creation

**Prod Nodes** - How many nodes produced this block

You can click on any of the aforementioned values to sort blocks by that value.

Click on the left-facing arrow to go back to the previous screen of the sidebar. Now click on the **Slowest Block** button to show additional information about the block with the highest latencies:


![3-1-1-Sidebar AllBlocks Chain](https://user-images.githubusercontent.com/60480123/225696997-3d05af32-c89f-47ee-aab7-2a0aef0db8e3.png)



On the top of the sidebar is the hash of the slowest block.

Right below the block’s hash are two tabs, **Chain** and **Network**.

By default, this page is opened on the **Chain** tab

**Date** - The block creation time

**Height** - The height of the block in the chain

**Global Slot** - The global slot this block was produced in since genesis

**Transactions** - Number of transaction

**Block producer** - The account of the block producer

**Max Receive Latency** - The maximal receive latency, i.e. the latest time the block arrived in a node since its creation  

Now click on the **Network** tab, which display all the nodes that have received this block.

![3-1-2-Sidebar AllBlocks Network](https://user-images.githubusercontent.com/60480123/225697842-7eff3a0a-33b9-47ac-8ab4-19b23515f86a.png)


**Node** - the name of the node that the block was received by

**Block Processing** - how long it took for this node to process this block

**Receive Latency** - how long it took since production time for this node to receive this block
