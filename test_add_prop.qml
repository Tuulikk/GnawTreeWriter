import QtQuick
Button {
    text: 'Click me'
}
property int newProp: 42
property string finalProp: 'it works!'
property bool insideProp: true

Rectangle {
    width: 100
    height: 100
    property string existingProp: "hello"

    Text {
        text: "test"
    }
}