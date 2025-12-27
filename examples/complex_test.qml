import QtQuick 2.15
import QtQuick.Controls 2.15

ApplicationWindow {
    visible: true
    width: 640
    height: 480
    title: "Complex QML Test"

    header: ToolBar {
        RowLayout {
            anchors.fill: parent
            ToolButton {
                text: "Back"
                onClicked: console.log("Back clicked")
            }
            Label {
                text: "Title"
                elide: Label.ElideRight
                horizontalAlignment: Qt.AlignHCenter
                verticalAlignment: Qt.AlignVCenter
                Layout.fillWidth: true
            }
        }
    }

    Rectangle {
        id: mainRect
        anchors.centerIn: parent
        width: 300
        height: 300
        color: "lightblue"

        Text {
            anchors.centerIn: parent
            text: "Hello World"
            font.pixelSize: 24

            MouseArea {
                anchors.fill: parent
                onClicked: {
                    console.log("Text clicked")
                    parent.color = "red"
                }
            }
        }
        
        property int customProp: 42
    }
}
