const std = @import("std");

pub fn main() !void {
    const stdout = std.io.getStdOut().writer();
    try stdout.print("Hello, {s}!\n", .{"World"});

    try greet("Zig");

    const result = add(5, 3);
    try stdout.print("5 + 3 = {d}\n", .{result});
}

fn greet(name: []const u8) !void {
    const stdout = std.io.getStdOut().writer();
    try stdout.print("Hello, {s}!\n", .{name});
}

fn add(a: i32, b: i32) i32 {
    return a + b;
}

fn multiply(a: i32, b: i32) i32 {
    return a * b;
}

const Point = struct {
    x: f32,
    y: f32,

    pub fn init(x: f32, y: f32) Point {
        return Point{ .x = x, .y = y };
    }

    pub fn distance(self: Point, other: Point) f32 {
        const dx = self.x - other.x;
        const dy = self.y - other.y;
        return @sqrt(dx * dx + dy * dy);
    }
};

test "basic addition" {
    const result = add(2, 3);
    try std.testing.expectEqual(@as(i32, 5), result);
}

test "multiply" {
    const result = multiply(4, 5);
    try std.testing.expectEqual(@as(i32, 20), result);
}
