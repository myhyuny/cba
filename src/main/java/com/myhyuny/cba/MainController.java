package com.myhyuny.cba;

import javafx.fxml.FXML;
import javafx.scene.control.ChoiceBox;
import javafx.scene.control.Label;
import javafx.scene.control.ProgressBar;
import javafx.scene.input.DragEvent;
import javafx.scene.layout.Pane;

import java.io.*;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.Paths;
import java.util.ArrayList;
import java.util.List;
import java.util.Map;
import java.util.concurrent.ExecutorService;
import java.util.regex.MatchResult;
import java.util.regex.Matcher;
import java.util.regex.Pattern;
import java.util.stream.IntStream;

import static java.lang.Runtime.getRuntime;
import static java.lang.StrictMath.*;
import static java.nio.file.Files.*;
import static java.util.Arrays.asList;
import static java.util.Arrays.stream;
import static java.util.Collections.emptyList;
import static java.util.concurrent.ForkJoinPool.commonPool;
import static java.util.function.Function.identity;
import static java.util.regex.Pattern.CASE_INSENSITIVE;
import static java.util.regex.Pattern.compile;
import static java.util.stream.Collectors.toList;
import static java.util.stream.Collectors.toMap;
import static javafx.application.Platform.runLater;
import static javafx.collections.FXCollections.observableList;
import static javafx.scene.input.TransferMode.MOVE;

/**
 * @author Hyunmin Kang
 */
public class MainController {

	private final ExecutorService background = commonPool();

	private final String command = stream(new String[] { "7z", "/usr/local/bin/7z", "/opt/local/bin/7z" })
			.filter((command -> {
				try {
					var p = getRuntime().exec(command);
					p.waitFor();
					return p.exitValue() == 0;
				} catch (IOException | InterruptedException e) {
					return false;
				}
			})).findFirst().orElse("");

	private final Pattern numberPattern = compile("(\\d+)");

	private boolean run = false;

	@FXML
	private Pane formPane;

	@FXML
	private ChoiceBox<Type> typeChoiceBox;

	@FXML
	private Label dropLabel;

	@FXML
	private ProgressBar progressBar;

	@FXML
	private Label messageLabel;

	@FXML
	private void initialize() {
		if (command.isEmpty()) {
			messageLabel.setText("Undefined 7z");
			formPane.setDisable(true);
			dropLabel.setDisable(true);
			run = true;
		} else {
			messageLabel.setText("");
		}

		typeChoiceBox.setItems(observableList(asList(Type.values())));
		typeChoiceBox.getSelectionModel().select(0);
	}

	@FXML
	private void handleOnDragOver(final DragEvent event) {
		if (!event.getDragboard().hasFiles()) {
			event.consume();
			return;
		}

		event.acceptTransferModes(MOVE);
	}

	@FXML
	private void handleOnDragDropped(final DragEvent event) {
		var dragboard = event.getDragboard();
		if (run || !dragboard.hasFiles()) {
			event.consume();
			return;
		}

		var files = dragboard.getFiles().stream().map(File::toPath).collect(toList());
		if (files.isEmpty()) {
			event.consume();
			return;
		}

		run = true;
		formPane.setDisable(true);
		dropLabel.setVisible(false);
		progressBar.setProgress(0.0);
		messageLabel.setText("Directory read");

		event.setDropCompleted(true);
		event.consume();

		var type = typeChoiceBox.getSelectionModel().getSelectedItem();
		background.execute(() -> compress(files, type));
	}

	private void compress(List<Path> paths, Type type) {
		var dirs = directories(paths);

		for (var dirIndex = 0; dirIndex < dirs.size(); dirIndex++) {
			var index = dirIndex;
			var dir = dirs.get(dirIndex);

			runLater(() -> messageLabel.setText((index + 1) + "/" + dirs.size() + " " + dir.getFileName()));

			var images = new ArrayList<Path>();
			try (var stream = newDirectoryStream(dir, this::extFilter)) {
				for (var path : stream) { images.add(path); }
			} catch (IOException e) {
				runLater(() -> messageLabel.setText(dir.getFileName() + ": " + e.getMessage()));
				break;
			}

			if (images.isEmpty()) {
				runLater(() -> messageLabel.setText(dir.getFileName() + " is empty."));
				break;
			}

			images.sort(this::imagesCompare);

			Type outputType;
			if (type != Type.Auto) {
				outputType = type;
			} else if (images.stream().mapToLong(this::fileSize).sum() > 16 * 1024 * 1024) {
				outputType = Type.Zip;
			} else {
				outputType = Type.SevenZip;
			}

			if (exists(Paths.get(dir.toString() + "." + outputType.ext))) { continue; }

			var numbers = (int) ceil(log10(images.size()));
			var format = "%0" + numbers + "d.jpg";
			var renamed = IntStream.range(0, images.size())
					.mapToObj(num -> String.format(format, num))
					.map(dir::resolve)
					.collect(toList());

			for (int i = 0, length = min(images.size(), renamed.size()); i < length; i++) {
				var source = images.get(i);
				var target = renamed.get(i);
				if (source.equals(target)) { break; }

				if (exists(target)) {
					runLater(() -> messageLabel.setText(target.getParent().getFileName() + "/" + target.getFileName() + " is exists."));
					break;
				}

				try {
					move(source, target);
				} catch (IOException e) {
					runLater(() -> messageLabel.setText(source.getFileName() + ": " + e.getMessage()));
					break;
				}
			}

			var cmd = new ArrayList<String>();
			cmd.add(command);
			cmd.add("a");
			cmd.add("-mx=9");
			cmd.add("-bb3");
			cmd.addAll(outputType.opts);
			cmd.add(dir.toString() + "." + outputType.ext);
			renamed.stream().map(Path::toString).forEach(cmd::add);

			var builder = new ProcessBuilder(cmd);
			try {
				var process = builder.start();
				readText(process.getInputStream());
				var err = readText(process.getErrorStream());
				process.waitFor();

				if (process.exitValue() != 0) {
					runLater(() -> messageLabel.setText(err));
					break;
				}

			} catch (IOException | InterruptedException | IllegalThreadStateException e) {
				runLater(() -> messageLabel.setText(builder + ": " + e.getMessage()));
				break;
			}

			runLater(() -> progressBar.setProgress((index + 1.0) / dirs.size()));
		}

		runLater(() -> {
			formPane.setDisable(false);
			dropLabel.setVisible(true);
			if (progressBar.getProgress() >= 1.0) { messageLabel.setText("Complete"); }
			run = false;
		});
	}

	private final Pattern extPattern = Pattern.compile(".+\\.(jpe?g)$", CASE_INSENSITIVE);

	private boolean extFilter(Path path) {
		return !isDirectory(path) && extPattern.matcher(path.toString()).find();
	}

	private List<Path> directories(final List<Path> paths) {
		return directories(paths, new ArrayList<>());
	}

	private int imagesCompare(Path f1, Path f2) {
		var r1 = toMatchResults(numberPattern.matcher(f1.getFileName().toString()));
		var r2 = toMatchResults(numberPattern.matcher(f2.getFileName().toString()));
		if (r1.size() < 1 || r2.size() < 1 || r1.size() != r2.size()) { return f1.compareTo(f2); }

		var i1 = r1.iterator();
		var i2 = r2.iterator();
		while (i1.hasNext() && i2.hasNext()) {
			var compare = Long.compare(Long.parseLong(i1.next().group(0)), Long.parseLong(i2.next().group(0)));
			if (compare != 0) { return compare; }
		}

		return f1.compareTo(f2);
	}

	private List<MatchResult> toMatchResults(Matcher matcher) {
		if (!matcher.find()) { return emptyList(); }

		var results = new ArrayList<MatchResult>();
		while (matcher.find()) { results.add(matcher.toMatchResult()); }

		return results;
	}

	private List<Path> directories(final List<Path> paths, final List<Path> dirs) {
		var entries = paths.stream()
				.filter(Files::isDirectory)
				.collect(toMap(identity(), this::getChildes)).entrySet().stream()
				.filter(e -> !e.getValue().isEmpty())
				.collect(toList());

		entries.stream().map(Map.Entry::getKey).forEach(dirs::add);

		var childs = entries.stream().flatMap(e -> e.getValue().stream()).collect(toList());
		if (childs.isEmpty()) { return dirs; }

		return directories(childs, dirs);
	}

	private List<Path> getChildes(Path dir) {
		try {
			var list = new ArrayList<Path>();
			for (var path : newDirectoryStream(dir)) { list.add(path); }
			return list;
		} catch (IOException e) {
			return emptyList();
		}
	}

	private long fileSize(Path path) {
		try {
			return Files.size(path);
		} catch (IOException e) {
			return 0;
		}
	}

	private String readText(InputStream is) {
		try (var in = new BufferedReader(new InputStreamReader(is))) {
			var builder = new StringBuilder();
			for (String str; (str = in.readLine()) != null; ) {
				builder.append(str).append('\n');
			}
			return builder.toString();
		} catch (IOException e) {
			return e.getMessage();
		}
	}

	private enum Type {
		Auto("Auto"),
		SevenZip("cb7", "-t7z", "-ms=on"),
		Zip("cbz", "-tzip");

		private final String ext;
		private final List<String> opts;

		Type(final String ext, final String... opts) {
			this.ext = ext;
			this.opts = List.of(opts);
		}
	}

}
