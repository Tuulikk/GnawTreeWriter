import QtQuick 2.15
import QtQuick.Controls 2.15

ApplicationWindow {
    visible: true
    width: 640
    height: 480
    title: "Complex QML Example"

    Rectangle {
        anchors.fill: parent
        color: "#f0f0f0"

        Text {
            id: titleText
            anchors.centerIn: parent
            text: "Hello, World!"
            font.pixelSize: 24
            color: "#ffffff"
        }

        Button {
            anchors.top: titleText.bottom
            anchors.horizontalCenter: parent.horizontalCenter
            anchors.margins: 20
            text: "Click Me"
            onClicked: {
                titleText.text = "Button Clicked!"
            }
        }

        Rectangle {
            width: 100
            height: 100
            color: "#00ff00"
            anchors.left: parent.left
            anchors.bottom: parent.bottom
            anchors.margins: 20
        }
    }
}
