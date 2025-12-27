package main

import "fmt"

type Coord float32
type TimeStamp uint64

// Shape represents either a Circle or Rectangle
type Shape struct {
	ShapeType string  // "Circle" or "Rectangle"
	Radius    Coord   // For Circle
	Width     Coord   // For Rectangle
	Height    Coord   // For Rectangle
}

// NewCircle creates a new Circle shape
func NewCircle(radius Coord) Shape {
	return Shape{
		ShapeType: "Circle",
		Radius:    radius,
	}
}

// NewRectangle creates a new Rectangle shape
func NewRectangle(width, height Coord) Shape {
	return Shape{
		ShapeType: "Rectangle",
		Width:     width,
		Height:    height,
	}
}

type PathSegment struct {
	BeginLocation    [2]Coord
	EndLocation      *[2]Coord
	BeginTime        TimeStamp
	EndTime          *TimeStamp
	BeginOrientation float32
	EndOrientation   *float32
}

type Animatable struct {
	ID    uint64
	Shape Shape
	Fill  bool
	Color [3]uint8
	Path  []PathSegment
}

// Message represents different message types
type Message struct {
	MessageType string      // "Begin", "Show", "Update", or "Hide"
	Timestamp   TimeStamp   // For Begin
	Animatable  *Animatable // For Show
	ID          uint64      // For Update and Hide
	PathSegment []PathSegment // For Update
}

// NewBeginMessage creates a Begin message
func NewBeginMessage(timestamp TimeStamp) Message {
	return Message{
		MessageType: "Begin",
		Timestamp:   timestamp,
	}
}

// NewShowMessage creates a Show message
func NewShowMessage(animatable Animatable) Message {
	return Message{
		MessageType: "Show",
		Animatable:  &animatable,
	}
}

// NewUpdateMessage creates an Update message
func NewUpdateMessage(id uint64, pathSegments []PathSegment) Message {
	return Message{
		MessageType: "Update",
		ID:          id,
		PathSegment: pathSegments,
	}
}

// NewHideMessage creates a Hide message
func NewHideMessage(id uint64) Message {
	return Message{
		MessageType: "Hide",
		ID:          id,
	}
}

func main() {
	fmt.Println("Hello, world!")
}
