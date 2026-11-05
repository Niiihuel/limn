import QtQuick
import QtQuick.Window
Window {
 id: window; visible: true; width:720; height:480; color: "#10121c"
 Item {
  id: output; width:720; height:480
  Item {
   id: scene; anchors.fill:parent
   Rectangle { anchors.fill:parent; gradient: Gradient { GradientStop {position:0;color:"#803d91"} GradientStop {position:0.5;color:"#146b92"} GradientStop {position:1;color:"#df714b"} } }
   Repeater { model:24; Rectangle { required property int index; x:index*32-30; y:0; width:12; height:480; color:index%2?"#30ffffff":"#50000000"; rotation:15 } }
   Rectangle { x:70;y:100;width:230;height:230;radius:115;color:"#f0b143" }
   Rectangle { x:440;y:130;width:180;height:230;radius:20;color:"#b3275a";rotation:-15 }
   Repeater { model:8; Text { required property int index; x:24;y:20+index*59;text:index%2?"Aurora · liquid glass 012345":"limn   music / library / explore";font.pixelSize:25;font.bold:true;color:index%2?"#14151a":"#ffffff" } }
  }
  ShaderEffectSource { id:sharpSource; sourceItem:scene; textureSize:Qt.size(720,480); live:true; visible:false; smooth:true }
  ShaderEffectSource { id:small; sourceItem:scene; textureSize:Qt.size(180,120); live:true; visible:false; smooth:true }
  ShaderEffect { id:horizontal; anchors.fill:parent;visible:false; property var source:small;property vector2d texelStep:Qt.vector2d(1/180,0);property real sigma:2;fragmentShader:"blur.frag.qsb" }
  ShaderEffectSource { id:blurredH;sourceItem:horizontal;textureSize:Qt.size(180,120);live:true;visible:false;smooth:true }
  ShaderEffect { id:vertical;anchors.fill:parent;visible:false;property var source:blurredH;property vector2d texelStep:Qt.vector2d(0,1/120);property real sigma:2;fragmentShader:"blur.frag.qsb" }
  ShaderEffectSource { id:frost;sourceItem:vertical;textureSize:Qt.size(180,120);live:true;visible:false;smooth:true }
  Item { id:glassLayer;anchors.fill:parent;visible:false
   Repeater { model:[{x:40,y:48,w:360,h:112,r:56,p:2},{x:245,y:205,w:390,h:180,r:32,p:4},{x:500,y:54,w:130,h:100,r:24,p:4}]
    ShaderEffect {
     required property var modelData
     x:modelData.x;y:modelData.y;width:modelData.w;height:modelData.h
     property var source:frost; property var sharp:sharpSource
     property vector2d itemSize:Qt.vector2d(width,height)
     property vector4d mapping:Qt.vector4d(x/720,y/480,width/720,height/480)
     property vector4d tint:Qt.vector4d(38/255,38/255,40/255,122/255)
     property vector4d material:Qt.vector4d(32,0.5,2.2,0)
     property real radius:modelData.r;property real power:modelData.p;property real dpr:1
     property vector4d glow:Qt.vector4d(0.5,0.5,0,1);property vector4d lens:Qt.vector4d(1,0,1,0);property vector4d tail:Qt.vector4d(0,0,0,0)
     fragmentShader:"glass.frag.qsb"
    }
   }
  }
 }
 Timer {interval:1000;running:true;onTriggered:scene.grabToImage(function(r){r.saveToFile("backdrop.png");glassLayer.visible=true;finish.start();},Qt.size(720,480)) }
 Timer {id:finish;interval:1000;onTriggered:output.grabToImage(function(r){r.saveToFile("aurora.png");Qt.quit();},Qt.size(720,480))}
}
