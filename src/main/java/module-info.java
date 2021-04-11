module com.myhyuny.cba {
	requires javafx.controls;
	requires javafx.fxml;
	requires java.desktop;

	opens com.myhyuny.cba to javafx.fxml;

	exports com.myhyuny.cba;
}