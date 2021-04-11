package com.myhyuny.cba;

import javafx.application.Application;
import javafx.scene.Parent;
import javafx.scene.Scene;
import javafx.stage.Stage;

import static javafx.fxml.FXMLLoader.load;

/**
 * @author Hyunmin Kang
 */
public class CBArchiver extends Application {

	public static void main(String[] args) { launch(args); }

	@Override
	public void start(Stage stage) throws Exception {
		var url = getClass().getResource("MainController.fxml");
		Parent root = load(url);
		stage.setScene(new Scene(root));
		stage.setTitle("Comic Book Archiver");
		stage.show();
	}

}
