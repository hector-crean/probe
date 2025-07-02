# Probe


![Rerun Probe](assets/rerun-probe.png)

Rerun's platform includes a movable 'camera', which renders its view to a render target. We can then sample this
render target to see the pixel colours at particular points on the render target. 

This repo is an attempt to recreate this in bevy.

![Probe Demo](assets/probe.gif)

The most complex element is implementing 'readback' of the sample pixels from the gpu. 

There are several application to this setup. We could, for instance, render meshes in the scene to a particular render layer with specific colours, and then do 'gpu picking', rather than having to do raycasting on the cpu. 


