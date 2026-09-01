package fixtures;

@interface Marker {}

sealed interface Shape permits Circle {}

@Marker
record Circle(int radius) implements Shape {}
